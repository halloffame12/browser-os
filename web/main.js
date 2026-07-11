// ============================================================================
// BrowserOS v0.4 — Capability-Secure Host Runtime (Typestate + WASI + Snapshot + Distributed)
// ============================================================================
// TCB BOUNDARY: This file is UNTRUSTED. All security-sensitive operations
// are gated by capability tokens validated inside the Rust kernel. This JS
// code can request operations, but the kernel decides whether to authorize
// them based on the presented capability.
//
// Capability Model:
//   - Each kernel operation requires a u32 capability token (slot index)
//   - The kernel minted a ROOT CAP at boot with ALL rights
//   - The shell derives attenuated caps from root for specific operations
//   - Syscall returns < 0 on capability failure (-2 = bad cap, -3 = insufficient rights, -4 = revoked)
// ============================================================================

let kernel = null;

// ============================================================================
// CAPABILITY MANAGER
// ============================================================================
// Manages the shell's capability hierarchy. The shell holds a root capability
// and derives child capabilities for specific file operations.

class CapManager {
  constructor(kernel) {
    this.kernel = kernel;
    this.rootCap = null;
  }

  setRootCap(slot) {
    this.rootCap = slot;
  }

  getRoot() {
    if (this.rootCap === null) {
      throw new Error("No root capability available");
    }
    return this.rootCap;
  }

  mint(parentSlot, objectType, objectId, rightsFlags) {
    // Use the human-readable name internally for compatibility
    return this.kernel.cap_mint(parentSlot, objectType, objectId, rightsFlags);
  }

  revoke(capSlot) {
    return this.kernel.cap_revoke(capSlot);
  }

  destroy(capSlot) {
    return this.kernel.cap_destroy(capSlot);
  }

  info(capSlot) {
    return this.kernel.cap_info(capSlot);
  }

  listAll() {
    return this.kernel.cap_list();
  }
}

// ============================================================================
// WASM LINEAR MEMORY BUFFER
// ============================================================================
// Provides efficient raw memory access for zero-copy sys_cap_write calls.
// The JS caller writes data into a pre-allocated WASM buffer, then passes
// the byte offset and length to the Rust kernel.

class WasmMemory {
  constructor(kernel) {
    this.kernel = kernel;
    this.buffer = null;
    this.offset = 0;
    this.capacity = 65536; // 64KB scratch buffer
    this.initialized = false;
  }

  init() {
    // Allocate a scratch buffer in WASM linear memory
    // We use a simple approach: reserve memory at a known offset
    // by growing the WASM memory if needed.
    // In production, you'd use wasm-bindgen's exported alloc function.
    this.buffer = new Uint8Array(this.kernel.memory.buffer);
    this.offset = this.buffer.byteLength - this.capacity;
    if (this.offset < 0) {
      this.offset = 0;
    }
    this.initialized = true;
  }

  writeString(str) {
    if (!this.initialized) this.init();
    const encoder = new TextEncoder();
    const bytes = encoder.encode(str);
    const dst = new Uint8Array(this.kernel.memory.buffer, this.offset, bytes.length);
    dst.set(bytes);
    return { offset: this.offset, length: bytes.length };
  }

  writeBytes(bytes) {
    if (!this.initialized) this.init();
    const dst = new Uint8Array(this.kernel.memory.buffer, this.offset, bytes.length);
    dst.set(bytes);
    return { offset: this.offset, length: bytes.length };
  }

  readString(offset, length) {
    const bytes = new Uint8Array(this.kernel.memory.buffer, offset, length);
    return new TextDecoder().decode(bytes);
  }
}

// ============================================================================
// DEVICE DRIVERS (Unchanged from v0.2)
// ============================================================================

class Display {
  constructor(terminalElement) {
    this.element = terminalElement;
    this.buffer = "";
  }

  write(text) {
    this.buffer += text;
    this.element.textContent = this.buffer;
    this.element.parentElement.scrollTop = this.element.parentElement.scrollHeight;
  }

  clear() {
    this.buffer = "";
    this.element.textContent = "";
  }

  getBuffer() {
    return this.buffer;
  }
}

class Keyboard {
  constructor() {
    this.inputBuffer = "";
    this.listeners = [];
  }

  onInput(callback) {
    this.listeners.push(callback);
  }

  emit(data) {
    this.inputBuffer += data;
    for (let listener of this.listeners) {
      listener(data);
    }
  }

  readLine() {
    const line = this.inputBuffer;
    this.inputBuffer = "";
    return line;
  }
}

class Storage {
  constructor() {
    this.dbName = "BrowserOS";
    this.storeName = "filesystem";
    this.db = null;
  }

  async init() {
    return new Promise((resolve, reject) => {
      const req = indexedDB.open(this.dbName, 1);
      req.onerror = () => reject(req.error);
      req.onsuccess = () => { this.db = req.result; resolve(); };
      req.onupgradeneeded = (e) => {
        const db = e.target.result;
        if (!db.objectStoreNames.contains(this.storeName)) {
          db.createObjectStore(this.storeName);
        }
      };
    });
  }

  async save(key, value) {
    return new Promise((resolve, reject) => {
      if (!this.db) { reject(new Error("DB not init")); return; }
      const tx = this.db.transaction([this.storeName], "readwrite");
      const store = tx.objectStore(this.storeName);
      const req = store.put(value, key);
      req.onerror = () => reject(req.error);
      req.onsuccess = () => resolve();
    });
  }

  async load(key) {
    return new Promise((resolve, reject) => {
      if (!this.db) { reject(new Error("DB not init")); return; }
      const tx = this.db.transaction([this.storeName], "readonly");
      const store = tx.objectStore(this.storeName);
      const req = store.get(key);
      req.onerror = () => reject(req.error);
      req.onsuccess = () => resolve(req.result);
    });
  }
}

class Timer {
  constructor() {
    this.startTime = Date.now();
  }

  getCurrentTime() {
    return Date.now() - this.startTime;
  }

  sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// ============================================================================
// SHELL — Capability-Aware Command Executor
// ============================================================================

class Shell {
  constructor(kernel, caps, wasmMem, display, keyboard, storage, timer, snapshotMgr, ambassador) {
    this.kernel = kernel;
    this.caps = caps;
    this.wasmMem = wasmMem;
    this.display = display;
    this.keyboard = keyboard;
    this.storage = storage;
    this.timer = timer;
    this.snapshotMgr = snapshotMgr || null;
    this.ambassador = ambassador || null;
    this.currentPath = "/";
    this.redirectOutput = null;
  }

  async execute(cmdLine) {
    // Handle output redirection: "command > file"
    let cmd = cmdLine;
    this.redirectOutput = null;

    if (cmdLine.includes(">")) {
      const parts = cmdLine.split(">");
      cmd = parts[0].trim();
      this.redirectOutput = parts[1].trim();
    }

    const parts = cmd.trim().split(/\s+/);
    if (parts.length === 0) return;

    const command = parts[0];
    const args = parts.slice(1);

    try {
      let output = "";
      let isCapabilityCommand = false;

      switch (command) {
        // ── LEGACY COMMANDS (v0.2 backward compat, ambient auth) ──────
        case "help":
          output = this.kernel.handle_command("help");
          break;

        case "clear":
          this.display.clear();
          return;

        case "uname":
          output = this.kernel.uname();
          break;

        case "uptime":
          output = this.kernel.handle_command("uptime");
          break;

        case "ls":
          output = await this.cmdLS(args);
          break;

        case "cat":
          output = await this.cmdCAT(args);
          break;

        case "echo":
          output = args.join(" ");
          break;

        case "touch":
          output = await this.cmdTOUCH(args);
          break;

        case "mkdir":
          output = await this.cmdMKDIR(args);
          break;

        case "pwd":
          output = this.currentPath;
          break;

        case "cd":
          output = await this.cmdCD(args);
          break;

        case "ps":
          output = `PID  STATE       TYPE\n---  -----       ----\n  0  [RUNNING]   init\n`;
          break;

        // ── CAPABILITY COMMANDS (v0.3) ─────────────────────────────────
        case "cap_root":
          output = `Root capability slot: ${this.caps.getRoot()}`;
          isCapabilityCommand = true;
          break;

        case "cap_info":
          output = await this.cmdCapInfo(args);
          isCapabilityCommand = true;
          break;

        case "cap_list":
          output = this.caps.listAll();
          isCapabilityCommand = true;
          break;

        case "cap_revoke":
          output = await this.cmdCapRevoke(args);
          isCapabilityCommand = true;
          break;

        case "cap_open":
          output = await this.cmdCapOpen(args);
          isCapabilityCommand = true;
          break;

        case "cap_write":
          output = await this.cmdCapWrite(args);
          isCapabilityCommand = true;
          break;

        case "cap_read":
          output = await this.cmdCapRead(args);
          isCapabilityCommand = true;
          break;

        case "cap_create":
          output = await this.cmdCapCreate(args);
          isCapabilityCommand = true;
          break;

         case "cap_ls":
           output = await this.cmdCapLS(args);
           isCapabilityCommand = true;
           break;

         // ── SNAPSHOT COMMANDS (v0.4) ────────────────────────────────────
         case "snap":
           output = await this.cmdSnap(args);
           break;

         case "restore":
           output = await this.cmdRestore(args);
           break;

         case "snap_list":
           output = await this.cmdSnapList(args);
           break;

         // ── WASI COMMANDS (v0.4) ────────────────────────────────────────
         case "wasi_init":
           output = await this.cmdWasiInit(args);
           break;

         case "wasi_root":
           output = this.kernel.wasi_get_root_fd();
           break;

         // ── DISTRIBUTED CAPABILITY COMMANDS (v0.4) ──────────────────────
         case "delegate":
           output = await this.cmdDelegate(args);
           break;

         case "import":
           output = await this.cmdImport(args);
           break;

         case "proxy_list":
           output = this.ambassador ? this.ambassador.listRemoteProxies() : "Ambassador not initialized";
           break;

         case "delegation_list":
           output = this.ambassador ? this.ambassador.listDelegations() : "Ambassador not initialized";
           break;

         default:
           output = `Command not found: ${command}`;
      }

      // Handle output redirection for legacy commands only
      if (this.redirectOutput && !isCapabilityCommand) {
        await this.cmdWriteFileLegacy(this.redirectOutput, output);
        output = `Wrote output to ${this.redirectOutput}`;
      }

      if (output) {
        this.display.write(output + "\n");
      }
    } catch (error) {
      this.display.write(`Error: ${error.message}\n`);
    }
  }

  // ── LEGACY FILE COMMANDS (ambient authority, for backward compat) ───

  async cmdLS(args) {
    const path = args.length > 0 ? args[0] : this.currentPath;
    const exists = this.kernel.fs_exists(path);
    if (exists !== 1) {
      if (exists === 0) return `ls: cannot access '${path}': Not a directory`;
      return `ls: cannot access '${path}': No such file or directory`;
    }
    const list = this.kernel.fs_list(path);
    if (!list) return "(empty)";
    return list.split(",").join("\n");
  }

  async cmdCAT(args) {
    if (args.length === 0) return "cat: missing operand";
    const path = args[0];
    try {
      return this.kernel.fs_cat(path);
    } catch (e) {
      return `cat: ${path}: No such file or directory`;
    }
  }

  async cmdTOUCH(args) {
    if (args.length === 0) return "touch: missing file operand";
    const path = args[0];
    const result = this.kernel.fs_create(path, false);
    if (result === 0) return `Created file: ${path}`;
    return `touch: cannot create '${path}'`;
  }

  async cmdMKDIR(args) {
    if (args.length === 0) return "mkdir: missing operand";
    const path = args[0];
    const result = this.kernel.fs_create(path, true);
    if (result === 0) return `Created directory: ${path}`;
    return `mkdir: cannot create '${path}'`;
  }

  async cmdCD(args) {
    if (args.length === 0) { this.currentPath = "/"; return ""; }
    const path = args[0];
    const exists = this.kernel.fs_exists(path);
    if (exists === 1) { this.currentPath = path; return ""; }
    if (exists === 0) return `cd: ${path}: Not a directory`;
    return `cd: ${path}: No such file or directory`;
  }

  async cmdWriteFileLegacy(path, content) {
    try {
      const fd = this.kernel.fs_create(path, false);
      if (fd === 0) {
        const openFd = this.kernel.fs_open(path, "w");
        if (openFd >= 0) {
          const bytes = Array.from(content).map(c => c.charCodeAt(0)).join(",");
          this.kernel.fs_write(openFd, bytes);
          this.kernel.fs_close(openFd);
        }
      }
    } catch (e) { /* ignore */ }
  }

  // ── CAPABILITY-GATED COMMANDS ───────────────────────────────────────

  async cmdCapInfo(args) {
    if (args.length === 0) return "cap_info: missing slot number";
    const slot = parseInt(args[0], 10);
    if (isNaN(slot)) return `cap_info: invalid slot '${args[0]}'`;
    return this.caps.info(slot);
  }

  async cmdCapRevoke(args) {
    if (args.length === 0) return "cap_revoke: missing slot number";
    const slot = parseInt(args[0], 10);
    if (isNaN(slot)) return `cap_revoke: invalid slot '${args[0]}'`;
    const result = this.caps.revoke(slot);
    if (result === 0) {
      return `Revoked all capabilities to object at slot ${slot}`;
    }
    return `cap_revoke: error ${result}`;
  }

  async cmdCapOpen(args) {
    // Usage: cap_open <path> [flags]
    // flags: 1=read, 2=write, 3=read+write
    if (args.length === 0) return "cap_open: missing path";
    const path = args[0];
    const flags = args.length > 1 ? parseInt(args[1], 10) : 1; // default: read
    const rootSlot = this.caps.getRoot();
    const result = this.kernel.sys_cap_open(rootSlot, path, flags);
    if (result >= 0) {
      return `Opened '${path}' → new capability slot ${result}`;
    }
    const errMsg = this.capErrorToString(result);
    return `cap_open: ${errMsg} (${result})`;
  }

  async cmdCapWrite(args) {
    // Usage: cap_write <cap_slot> <text>
    if (args.length < 2) return "cap_write: missing arguments";
    const capSlot = parseInt(args[0], 10);
    if (isNaN(capSlot)) return `cap_write: invalid cap slot '${args[0]}'`;
    const text = args.slice(1).join(" ");

    // Write data into WASM linear memory for zero-copy transfer
    const { offset, length } = this.wasmMem.writeString(text);
    const result = this.kernel.sys_cap_write(capSlot, offset, length);
    if (result >= 0) {
      return `Wrote ${result} bytes to file (cap ${capSlot})`;
    }
    return `cap_write: error ${result}`;
  }

  async cmdCapRead(args) {
    if (args.length === 0) return "cap_read: missing cap slot";
    const capSlot = parseInt(args[0], 10);
    if (isNaN(capSlot)) return `cap_read: invalid cap slot '${args[0]}'`;

    const content = this.kernel.sys_cap_read(capSlot);
    if (content.startsWith("Error: ")) {
      return content;
    }
    return content;
  }

  async cmdCapCreate(args) {
    // Usage: cap_create <path> [dir]
    if (args.length === 0) return "cap_create: missing path";
    const path = args[0];
    const isDir = args.length > 1 && args[1] === "dir";
    const rootSlot = this.caps.getRoot();
    const result = this.kernel.sys_cap_create(rootSlot, path, isDir);
    if (result >= 0) {
      return `Created '${path}' → new capability slot ${result}`;
    }
    return `cap_create: error ${result}`;
  }

  async cmdCapLS(args) {
    const path = args.length > 0 ? args[0] : this.currentPath;
    const rootSlot = this.caps.getRoot();

    // Use the capability-gated list syscall
    const result = this.kernel.sys_cap_list(rootSlot, path);
    if (result.startsWith("Error: ")) {
      return result;
    }
    if (!result) {
      return "(empty)";
    }
    return result.split(",").join("\n");
  }

  // ── SNAPSHOT COMMANDS ──────────────────────────────────────────────

  async cmdSnap(args) {
    if (!this.snapshotMgr) return "Snapshot manager not initialized";
    const tag = args[0] || undefined;
    try {
      const result = await this.snapshotMgr.snapshot(tag);
      return `Snapshot saved: ${result}`;
    } catch (e) {
      return `snap error: ${e.message}`;
    }
  }

  async cmdRestore(args) {
    if (!this.snapshotMgr) return "Snapshot manager not initialized";
    if (args.length === 0) return "restore: missing tag";
    try {
      const result = await this.snapshotMgr.restore(args[0]);
      return `Restored from snapshot: ${args[0]}`;
    } catch (e) {
      return `restore error: ${e.message}`;
    }
  }

  async cmdSnapList(args) {
    if (!this.snapshotMgr) return "Snapshot manager not initialized";
    const tags = await this.snapshotMgr.list();
    return tags.length > 0 ? tags.join("\n") : "(no snapshots)";
  }

  // ── WASI COMMANDS ────────────────────────────────────────────────────

  async cmdWasiInit(args) {
    const result = this.kernel.wasi_init_root();
    if (result === 0) {
      return "WASI table initialized (fd 3 = root)";
    }
    return `wasi_init_root error: ${result}`;
  }

  // ── DISTRIBUTED CAPABILITY COMMANDS ──────────────────────────────────

  async cmdDelegate(args) {
    if (!this.ambassador) return "CapabilityAmbassador not initialized";
    if (args.length < 2) return "delegate: usage: delegate <peerId> <capSlot>";
    const peerId = args[0];
    const capSlot = parseInt(args[1], 10);
    if (isNaN(capSlot)) return `delegate: invalid cap slot '${args[1]}'`;
    try {
      const token = await this.ambassador.delegateCap(peerId, capSlot);
      return `Delegated cap ${capSlot} to ${peerId}: ${token}`;
    } catch (e) {
      return `delegate error: ${e.message}`;
    }
  }

  async cmdImport(args) {
    if (!this.ambassador) return "CapabilityAmbassador not initialized";
    if (args.length < 2) return "import: usage: import <peerId> <secret>";
    const peerId = args[0];
    const secret = args.slice(1).join(" ");
    try {
      const offer = await this.ambassador.createOffer(peerId, secret);
      return `Offer created for peer ${peerId}. SDP type: ${offer.signaling.type}\nUse acceptOffer on the remote side with this signaling data.`;
    } catch (e) {
      return `import error: ${e.message}`;
    }
  }

  // ── HELPERS ──────────────────────────────────────────────────────────

  capErrorToString(code) {
    const errors = {
      "-2": "Bad capability",
      "-3": "Insufficient rights",
      "-4": "Capability revoked",
      "-5": "Object not found",
      "-6": "Path not found",
      "-7": "Not a directory",
      "-8": "Not a file",
      "-9": "Is a directory",
      "-10": "Name too long",
      "-11": "Parent not found",
      "-12": "Inode not found",
      "-13": "FD not found",
      "-14": "Invalid UTF-8",
      "-15": "Kernel not booted",
    };
    return errors[code.toString()] || `Unknown error ${code}`;
  }
}

// ============================================================================
// MAIN INITIALIZATION
// ============================================================================

async function main() {
  // Wait for DOM
  while (document.readyState !== "complete") {
    await new Promise(r => setTimeout(r, 10));
  }

  const terminalDiv = document.getElementById("terminal");
  const inputElem = document.getElementById("input");

  if (!terminalDiv || !inputElem) {
    throw new Error("DOM elements not found");
  }

  // Initialize device drivers
  const display = new Display(terminalDiv);
  const keyboard = new Keyboard();
  const storage = new Storage();
  const timer = new Timer();

  await storage.init();

  try {
    // Import and initialize WASM kernel module
    const browserOS = await import("./browser_os.js");
    const init = browserOS.default;
    await init();

    kernel = browserOS;

    // Boot the kernel
    const currentTime = BigInt(timer.getCurrentTime());
    const bootMsg = kernel.boot(currentTime);
    display.write(bootMsg);

    // Retrieve the root capability slot from the kernel
    const rootCapSlot = kernel.cap_get_root();

    // Initialize capability manager
    const caps = new CapManager(kernel);
    caps.setRootCap(rootCapSlot);

    // Initialize WASM memory buffer for zero-copy writes
    const wasmMem = new WasmMemory(kernel);

    // Initialize v0.4 subsystems
    const { SnapshotManager } = await import("./snapshot.js");
    const snapshotMgr = new SnapshotManager(kernel);

    const { CapabilityAmbassador } = await import("./webrtc_ipc.js");
    const ambassador = new CapabilityAmbassador(kernel, wasmMem, (peerId, capSlot) => {
      display.write(`[Ambassador] Remote cap ${capSlot} received from ${peerId}\n`);
    });

    // Create shell with capability awareness (v0.4)
    const shell = new Shell(kernel, caps, wasmMem, display, keyboard, storage, timer, snapshotMgr, ambassador);

    // Update kernel time periodically
    setInterval(() => {
      kernel.update_time(BigInt(timer.getCurrentTime()));
    }, 100);

    // Display boot signature with capability info
    display.write(`Root authority: cap slot ${rootCapSlot} (ALL rights to inode 0)\n`);
    display.write(`Use 'cap_list' to see all capabilities, 'cap_info <slot>' for details.\n\n`);

    // Terminal input handling
    inputElem.addEventListener("keydown", async (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        const cmd = inputElem.value;
        inputElem.value = "";

        display.write("> " + cmd + "\n");

        try {
          await shell.execute(cmd);
        } catch (e) {
          display.write(`Error: ${e.message}\n`);
        }

        // Show prompt with current path
        if (shell.currentPath === "/") {
          inputElem.placeholder = "→ ";
        } else {
          inputElem.placeholder = shell.currentPath + " → ";
        }
      }
    });

    inputElem.placeholder = "→ ";
    inputElem.focus();

  } catch (e) {
    console.error("Boot error:", e);
    const errorDiv = document.getElementById("terminal");
    if (errorDiv) {
      errorDiv.textContent = `Boot error: ${e.message}\n\n${e.stack || ""}\n\n(Check F12 console)`;
    }
  }
}

main().catch(e => {
  console.error("Fatal:", e);
  const errorDiv = document.getElementById("terminal");
  if (errorDiv) {
    errorDiv.textContent = `Fatal error: ${e.message}`;
  }
});
