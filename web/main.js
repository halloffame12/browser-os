// ============================================================================
// KERNEL INTERFACE
// ============================================================================

let kernel = null;  // Will hold WASM module

// ============================================================================
// DEVICE DRIVERS
// ============================================================================

class Display {
  constructor(terminalElement) {
    this.element = terminalElement;
    this.buffer = "";
  }
  
  write(text) {
    this.buffer += text;
    this.element.textContent = this.buffer;
    // Auto-scroll to bottom
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
      req.onsuccess = () => {
        this.db = req.result;
        resolve();
      };
      
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
      if (!this.db) {
        reject(new Error("Database not initialized"));
        return;
      }
      
      const tx = this.db.transaction([this.storeName], "readwrite");
      const store = tx.objectStore(this.storeName);
      const req = store.put(value, key);
      
      req.onerror = () => reject(req.error);
      req.onsuccess = () => resolve();
    });
  }
  
  async load(key) {
    return new Promise((resolve, reject) => {
      if (!this.db) {
        reject(new Error("Database not initialized"));
        return;
      }
      
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
// SHELL COMMAND EXECUTOR
// ============================================================================

class Shell {
  constructor(kernel, display, keyboard, storage, timer) {
    this.kernel = kernel;
    this.display = display;
    this.keyboard = keyboard;
    this.storage = storage;
    this.timer = timer;
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
      
      switch (command) {
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
          output = "Process table:\nPID 0: init [RUNNING]";
          break;
        
        default:
          output = `Command not found: ${command}`;
      }
      
      if (this.redirectOutput) {
        await this.cmdWriteFile(this.redirectOutput, output);
        output = `Wrote output to ${this.redirectOutput}`;
      }
      
      if (output) {
        this.display.write(output + "\n");
      }
    } catch (error) {
      this.display.write(`Error: ${error.message}\n`);
    }
  }
  
  async cmdLS(args) {
    const path = args.length > 0 ? args[0] : this.currentPath;
    // First check if path exists and is a directory
    const exists = this.kernel.fs_exists(path);
    if (exists !== 1) {
      if (exists === 0) {
        return `ls: cannot access '${path}': Not a directory`;
      } else {
        return `ls: cannot access '${path}': No such file or directory`;
      }
    }
    // Path is a directory, list it
    const list = this.kernel.fs_list(path);
    if (!list) {
      return "(empty)";
    }
    return list.split(",").join("\n");
  }
  
  async cmdCAT(args) {
    if (args.length === 0) {
      return "cat: missing operand";
    }
    const path = args[0];
    try {
      return this.kernel.fs_cat(path);
    } catch (e) {
      return `cat: ${path}: No such file or directory`;
    }
  }
  
  async cmdTOUCH(args) {
    if (args.length === 0) {
      return "touch: missing file operand";
    }
    const path = args[0];
    const result = this.kernel.fs_create(path, false);
    if (result === 0) {
      return `Created file: ${path}`;
    } else {
      return `touch: cannot create '${path}'`;
    }
  }
  
  async cmdMKDIR(args) {
    if (args.length === 0) {
      return "mkdir: missing operand";
    }
    const path = args[0];
    const result = this.kernel.fs_create(path, true);
    if (result === 0) {
      return `Created directory: ${path}`;
    } else {
      return `mkdir: cannot create '${path}'`;
    }
  }
  
  async cmdCD(args) {
    if (args.length === 0) {
      this.currentPath = "/";
      return "";
    }
    const path = args[0];
    // Check if path exists and is a directory
    const exists = this.kernel.fs_exists(path);
    if (exists === 1) {
      this.currentPath = path;
      return "";
    } else if (exists === 0) {
      return `cd: ${path}: Not a directory`;
    } else {
      return `cd: ${path}: No such file or directory`;
    }
  }
  
  async cmdWriteFile(path, content) {
    try {
      const fd = this.kernel.fs_create(path, false);
      if (fd === 0) {
        const openFd = this.kernel.fs_open(path, "w");
        if (openFd >= 0) {
          // Convert string to comma-separated bytes
          const bytes = Array.from(content).map(c => c.charCodeAt(0)).join(",");
          this.kernel.fs_write(openFd, bytes);
          this.kernel.fs_close(openFd);
        }
      }
    } catch (e) {
      // Ignore file write errors in redirects
    }
  }
}

// ============================================================================
// MAIN INITIALIZATION
// ============================================================================

async function main() {
  // Wait for DOM to be fully ready
  while (document.readyState !== "complete") {
    await new Promise(r => setTimeout(r, 10));
  }
  
  console.log("1. Starting BrowserOS initialization...");
  
  const terminalDiv = document.getElementById("terminal");
  const inputElem = document.getElementById("input");
  
  if (!terminalDiv || !inputElem) {
    throw new Error("DOM elements not found: terminal=" + !!terminalDiv + " input=" + !!inputElem);
  }
  
  console.log("2. Terminal div found:", terminalDiv.id);
  console.log("3. Input elem found:", inputElem.id);
  
  // Initialize device drivers
  const display = new Display(terminalDiv);
  const keyboard = new Keyboard();
  const storage = new Storage();
  const timer = new Timer();
  
  console.log("4. Device drivers initialized");
  
  try {
    await storage.init();
    console.log("5. Storage initialized");
  } catch (e) {
    console.warn("Storage initialization failed:", e);
  }
  
  try {
    // Import and initialize kernel
    console.log("6. Importing kernel module...");
    const browserOS = await import("./browser_os.js");
    console.log("7. Kernel module imported, exports:", 
      Object.keys(browserOS).slice(0, 15).join(", "));
    
    const init = browserOS.default;
    if (typeof init !== "function") {
      throw new Error("init is not a function, got: " + typeof init);
    }
    console.log("8. Init function is callable");
    
    // Initialize the WASM module first
    console.log("9. Calling await init()...");
    const result = await init();
    console.log("10. WASM module initialized, result:", result);
    
    // Now kernel is available with all exported functions
    kernel = browserOS;
    console.log("11. Kernel object assigned");
    console.log("12. Checking kernel functions...");
    console.log("    boot:", typeof kernel.boot);
    console.log("    fs_list:", typeof kernel.fs_list);
    console.log("    fs_exists:", typeof kernel.fs_exists);
    console.log("    update_time:", typeof kernel.update_time);
    console.log("    fs_create:", typeof kernel.fs_create);
    
    if (typeof kernel.boot !== "function") {
      throw new Error("kernel.boot is not available, got: " + typeof kernel.boot);
    }
    
    // Boot the kernel - boot expects bigint, not regular number
    const currentTime = BigInt(timer.getCurrentTime());
    console.log("13. Calling boot(" + currentTime + "n)...");
    const bootMsg = kernel.boot(currentTime);
    console.log("14. Boot successful, got message of length", bootMsg.length);
    
    display.write(bootMsg);
    console.log("14. Boot message written to display");
    
    // Create shell
    const shell = new Shell(kernel, display, keyboard, storage, timer);
    console.log("15. Shell created");
    
    // Update kernel time periodically
    setInterval(() => {
      kernel.update_time(BigInt(timer.getCurrentTime()));
    }, 100);
    console.log("16. Timer interval set");
    
    // Terminal input handling
    inputElem.addEventListener("keydown", async (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        const cmd = inputElem.value;
        inputElem.value = "";
        
        // Display the command
        display.write("> " + cmd);
        
        // Execute command
        try {
          await shell.execute(cmd);
        } catch (e) {
          display.write(`Error executing command: ${e.message}\n`);
        }
        
        // Show prompt
        if (shell.currentPath === "/") {
          inputElem.placeholder = "$ ";
        } else {
          inputElem.placeholder = shell.currentPath + "$ ";
        }
      }
    });
    console.log("17. Input handler attached");
    
    // Initial prompt
    inputElem.placeholder = "$ ";
    inputElem.focus();
    console.log("18. BrowserOS fully initialized!");
    
  } catch (e) {
    console.error("ERROR during initialization:", e);
    console.error("Stack:", e.stack);
    document.getElementById("terminal").textContent = "Boot error: " + e.message + "\n\nStack:\n" + e.stack;
  }
}

main().catch(e => {
  console.error("Failed to boot BrowserOS:", e);
  const errorDiv = document.getElementById("terminal");
  if (errorDiv) {
    errorDiv.textContent = "Boot error: " + e.message + 
      "\n\nFull error details:\n" + (e.stack || "No stack trace available") +
      "\n\n(Check browser console with F12 for more information)";
  }
});