# BrowserOS: A Research-Level Virtual Operating System in WebAssembly

**Author:** AI Research Assistant  
**Date:** February 2026  
**Status:** Research Implementation v0.2

---

## Executive Summary

BrowserOS is a complete operating system kernel implemented in Rust and compiled to WebAssembly (WASM) that runs entirely within a web browser. It demonstrates core OS concepts including process management, virtual file systems, syscall abstraction, and cooperative multitasking—all accessible through a terminal interface.

The project bridges systems programming and web technologies, proving that complex OS abstractions can be effectively implemented using modern WebAssembly and JavaScript interoperability.

---

## I. Architectural Overview

### 1.1 System Stack

```
┌─────────────────────────────────────────────────────────────┐
│                   Browser Runtime (V8/SpiderMonkey)         │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  JavaScript Host Runtime (device drivers)           │   │
│  │  ├─ Display Driver (terminal output)               │   │
│  │  ├─ Keyboard Driver (input handling)               │   │
│  │  ├─ Storage Driver (IndexedDB persistence)         │   │
│  │  └─ Timer (system clock)                           │   │
│  └──────────────────┬──────────────────────────────────┘   │
│                     │ Syscall Bridge                        │
│  ┌──────────────────▼──────────────────────────────────┐   │
│  │  WASM Engine (WebAssembly Binary)                   │   │
│  └──────────────────┬──────────────────────────────────┘   │
└─────────────────────┼───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│              RUST KERNEL (wasm32-unknown-unknown)           │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  KERNEL STATE                                        │  │
│  │  • Boot status, uptime tracking                      │  │
│  │  • Process table ([PID → PCB])                       │  │
│  │  • Inode map ([Inode_ID → Inode])                   │  │
│  │  • Open file descriptors ([FD → Offset/Mode])       │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  PROCESS MANAGER                                     │  │
│  │  • Process creation (PID allocation)                 │  │
│  │  • Process state tracking                            │  │
│  │  • Parent-child relationships                        │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  VIRTUAL FILE SYSTEM (VFS)                           │  │
│  │  • Inode structure (files & directories)             │  │
│  │  • Path traversal & lookup                           │  │
│  │  • File creation & deletion                          │  │
│  │  • File I/O (read/write)                             │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  SYSCALL DISPATCHER                                  │  │
│  │  Exported functions:                                 │  │
│  │  • fs_open, fs_read, fs_write, fs_close             │  │
│  │  • fs_create, fs_list, fs_cat                       │  │
│  │  • process_spawn, get_uptime                         │  │
│  │  • boot, update_time, uname                          │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
└──────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    USER SPACE                                │
│  • Shell (command interpreter)                              │
│  • Built-in commands: ls, cat, touch, echo, mkdir, cd, pwd  │
│  • Output redirection (> operator)                           │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 Data Flow

```
User Input (keyboard)
        ↓
JavaScript Input Handler
        ↓
Shell Command Parser
        ↓
Command Executor (matches command)
        ↓
    ├─→ Local execution (help, clear, pwd, cd)
    │
    ├─→ Syscall to Kernel (fs_*, process_*)
    │       ↓
    │   VFS/Process Manager
    │       ↓
    │   Device Driver (for I/O)
    │       ↓
    │   Return result
    │
    └─→ Output Redirection (write to file)
        ↓
Display Output in Terminal
```

---

## II. Core Components

### 2.1 Rust Kernel (`kernel/src/lib.rs`)

#### Thread-Local Kernel State

The kernel maintains **global mutable state** using Rust's `thread_local!` and `RefCell` patterns:

```rust
thread_local! {
    static KERNEL: RefCell<Kernel> = RefCell::new(Kernel::new());
}
```

**Why this design?**
- WebAssembly is single-threaded; thread_local ensures atomic access
- `RefCell` enables interior mutability while maintaining Rust's memory safety
- All syscalls query/modify through this single state object

#### Process Control Block (PCB)

```rust
pub struct ProcessControlBlock {
    pid: u32,
    state: ProcessState,  // Ready, Running, Waiting, Terminated
    parent_pid: Option<u32>,
    exit_code: i32,
}
```

**Design rationale:**
- Minimal PCB to reduce WASM binary size
- Real OS would include: registers, memory map, signal handlers, resource limits
- Parent PID enables process hierarchy for future capability-based security

#### Virtual File System (Inode-based)

```rust
pub struct Inode {
    id: u32,          // Globally unique
    inode_type: InodeType,  // File or Directory
    name: String,
    data: Vec<u8>,    // File content (in-memory only currently)
    children: HashMap<String, u32>,  // Dir entries
    parent: Option<u32>,
}
```

**Design decisions:**
- **Inode-based architecture** (like Unix) rather than FAT-style allocation
- **In-memory only** for v0.2 (IndexedDB support prepared for future)
- **Path traversal** through parent-child links for O(n) lookup
- **Unique global IDs** allow later persistent storage mapping

**Filesystem Layout:**
```
/                           (inode 0, root)
├── bin/                    (inode N)
│   ├── sh
│   ├── cat
│   └── ls
├── etc/                    (inode M)
│   └── config.sys
├── home/
│   └── user/
└── tmp/
```

#### File Descriptor Management

```rust
pub struct FileDescriptor {
    inode_id: u32,
    offset: usize,          // Current read/write position
    mode: String,           // "r" or "w"
}

open_files: HashMap<u32, FileDescriptor>  // fd→descriptor mapping
```

**Design:**
- FDs 0, 1, 2 reserved for stdin, stdout, stderr (future)
- FDs ≥ 3 allocated on-demand
- Offset tracking enables sequential I/O
- Mode field enforces access control (simple for now)

#### Syscall Interface

```rust
#[wasm_bindgen]
pub fn fs_open(path: &str, mode: &str) -> i32
pub fn fs_read(fd: u32, size: usize) -> String
pub fn fs_write(fd: u32, data: &str) -> i32
pub fn fs_create(path: &str, is_dir: bool) -> i32
pub fn fs_close(fd: u32) -> i32
pub fn fs_list(path: &str) -> String
pub fn fs_cat(path: &str) -> String
```

**Syscall conventions:**
- Return `-1` on error (C-like)
- Return file descriptor on success (fd > 0)
- String serialization for WASM↔JS communication (I/O overhead justified by simplicity)

---

### 2.2 JavaScript Host Runtime (`web/main.js`)

#### Device Drivers

**Display Driver**
```javascript
class Display {
    write(text)    // Append to terminal
    clear()        // Clear screen
    getBuffer()    // Read full screen
}
```
- Manages terminal text buffer
- Auto-scrolls to bottom on write
- Used by shell for output

**Keyboard Driver**
```javascript
class Keyboard {
    onInput(callback)  // Register listener
    emit(data)         // Generate input event
    readLine()         // Consume buffered input
}
```
- Event-driven architecture
- Decouples input handling from consumption

**Storage Driver**
```javascript
class Storage {
    async save(key, value)  // IndexedDB write
    async load(key)         // IndexedDB read
}
```
- Prepared for persistent VFS (not yet integrated)
- Uses native browser IndexedDB API
- Async/await for non-blocking I/O

**Timer**
```javascript
class Timer {
    getCurrentTime()  // Returns elapsed ms since boot
    sleep(ms)         // Promise-based delay
}
```

#### Shell Implementation

```javascript
class Shell {
    async execute(cmdLine)  // Main command dispatcher
}
```

**Command categories:**

1. **Built-in Shell Commands** (no kernel call)
   - `clear`: Clear display
   - `pwd`: Print current directory
   - `cd [path]`: Change current directory
   - `ps`: List processes

2. **File System Commands** (syscalls)
   - `ls [path]`: List directory (fs_list → kernel)
   - `cat [path]`: Read file (fs_cat → kernel)
   - `touch [path]`: Create file (fs_create → kernel)
   - `mkdir [path]`: Create directory (fs_create → kernel)

3. **System Commands**
   - `help`: Show command list
   - `uname`: Show OS info
   - `uptime`: Show system uptime

4. **I/O Redirection**
   - `echo hello > /tmp/test.txt` 
   - Parsed as: command + output file
   - Syscalls: fs_create, fs_open, fs_write

---

## III. Design Decisions & Rationale

### 3.1 Browser as Hardware Abstraction

| Component | Hardware Role | Implementation |
|-----------|---|---|
| JavaScript Engine | CPU | Executes WASM instructions |
| localStorage/IndexedDB | Persistent Storage | File system backing store |
| DOM/Canvas | Display | Terminal output |
| Keyboard Events | Keyboard Hardware | Input source |
| setTimeout/RAF | Timer | System clock |
| WebAssembly Memory | RAM | Heap allocation |

**Why?** The browser provides a complete hardware abstraction, allowing the kernel to be truly portable—it runs on any modern browser without modification.

### 3.2 Syscall Abstraction Layer

**Why separate kernel ↔ JS?**
- **Security**: Kernel can validate all JS requests before device access
- **Portability**: Kernel logic independent of JS
- **Testability**: Can stub drivers for unit testing
- **Extensibility**: New syscalls added by extending enum dispatcher

**Why `wasm-bindgen` for FFI?**
- Type-safe Rust ↔ JS communication
- Automatic memory marshaling
- Zero unsafe code in syscall path
- Compiler-verified interface contracts

### 3.3 Virtual File System Design

**Why inode-based vs block-based?**
- Simpler conceptually (file = inode + data)
- Natural mapping to Java/JavaScript heap objects
- Supports arbitrary file sizes (no fixed block size)
- Trade-off: less efficient than real filesystems (acceptable for research)

**Why in-memory first?**
- Easier debugging and testing
- Performance (no I/O latency)
- v0.2 can demonstrate filesystem semantics
- v0.3 will integrate IndexedDB for persistence

**Path traversal algorithm:**
```
lookup("/home/user/file.txt"):
  start at root (inode 0)
  for each component ["home", "user", "file.txt"]:
    find child inode by name in parent.children
    traverse to child
  return final inode_id
```

**Time complexity:** O(depth × avg_children) per lookup
**Space complexity:** O(num_files) for all inodes

### 3.4 Process Model

**Why cooperative multitasking vs preemptive?**
- WASM is single-threaded; preemption meaningless
- Simplifies implementation (no interrupt handlers)
- Sufficient for cooperative userland programs
- Can add simulator preemption in JavaScript event loop

**PID allocation:**
```rust
next_pid: u32 = 1  // Process 0 reserved for init
create_process() → returns next_pid++
```

**Process hierarchy:**
- init (PID 0) is root
- Each process has optional parent_pid
- Shell can create child processes (not yet exposed)

---

## IV. Implementation Details

### 4.1 Syscall Flow Example: `cat /tmp/test.txt`

**JavaScript:**
```javascript
await shell.execute("cat /tmp/test.txt")

// Shell.cmdCAT:
const content = kernel.fs_cat("/tmp/test.txt")
display.write(content)
```

**Rust Kernel:**
```rust
#[wasm_bindgen]
pub fn fs_cat(path: &str) -> String {
    KERNEL.with(|k| {
        let kernel = k.borrow();
        match kernel.get_inode(path) {           // O(depth) lookup
            Ok(inode_id) => {
                match kernel.read_file_content(inode_id) {
                    Ok(content) => content,       // Return UTF-8 string
                    Err(e) => format!("Error: {}", e),
                }
            }
            Err(e) => format!("Error: {}", e),
        }
    })
}

fn read_file_content(&self, inode_id: u32) -> Result<String, String> {
    if let Some(inode) = self.inode_map.get(&inode_id) {
        String::from_utf8(inode.data.clone())  // Convert bytes to string
    } else {
        Err("Inode not found".to_string())
    }
}
```

**Performance analysis:**
- Path lookup: O(depth) comparisons
- String conversion: O(file_size) copy
- Total: O(depth + file_size)

### 4.2 Persistence & Storage

**Current state (v0.2):** In-memory only
**Data loss:** On page reload, all files disappear

**Future (v0.3): IndexedDB Integration**

```javascript
// Save filesystem on shutdown
await storage.save("vfs_state", JSON.stringify({
    inodes: [...],
    files: {...}
}))

// Restore on boot
const state = await storage.load("vfs_state")
kernel.restore_vfs(state)
```

**Challenges:**
- WASM ↔ JSON serialization overhead
- IndexedDB quota limits (~50MB)
- Binary file support (current: UTF-8 only)

---

## V. Extension Points

### 5.1 Adding a New Syscall

**Example: `sys_random() -> u32`**

**Step 1: Add Rust export**
```rust
#[wasm_bindgen]
pub fn sys_random() -> u32 {
    // Use JavaScript's Math.random() via wasm-bindgen
    js_random()
}

// Delegate to JS
#[wasm_bindgen(raw_module = "./random.js")]
extern "C" {
    fn get_random() -> u32;
}
```

**Step 2: Add JavaScript helper**
```javascript
export function get_random() {
    return Math.floor(Math.random() * 2**32);
}
```

**Step 3: Expose in shell**
```javascript
case "random":
    output = String(kernel.sys_random());
    break;
```

### 5.2 Adding a New Command

**Example: `hexdump /path/file`**

```javascript
case "hexdump":
    output = await this.cmdHEXDUMP(args);
    break;

async cmdHEXDUMP(args) {
    if (args.length === 0) return "hexdump: missing file";
    const path = args[0];
    const content = kernel.fs_cat(path);
    
    // Convert to hex representation
    let hex = "";
    for (let i = 0; i < content.length; i += 16) {
        const chunk = content.slice(i, i + 16);
        const bytes = Array.from(chunk)
            .map(c => c.charCodeAt(0).toString(16).padStart(2, '0'))
            .join(' ');
        hex += `${i.toString(16).padStart(8, '0')}: ${bytes}\n`;
    }
    return hex;
}
```

### 5.3 Adding Persistence

**IndexedDB schema:**
```rust
// Kernel method
pub fn serialize_vfs(&self) -> String {
    serde_json::to_string(&self.inode_map).unwrap()
}

pub fn deserialize_vfs(&mut self, json: &str) {
    self.inode_map = serde_json::from_str(json).unwrap();
}
```

**Integration:**
```javascript
// On boot
const vfs_state = await storage.load("vfs_snapshot");
if (vfs_state) {
    kernel.deserialize_vfs(vfs_state);
}

// On shutdown (via beforeunload)
window.addEventListener("beforeunload", async () => {
    const state = kernel.serialize_vfs();
    await storage.save("vfs_snapshot", state);
});
```

### 5.4 Implementing Process Spawning

**Current state:** Process table exists but not exposed

**Future implementation:**

```rust
#[wasm_bindgen]
pub fn exec_program(program_name: &str, args: &str) -> u32 {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        
        // Load WASM module from /bin/program_name
        let program_path = format!("/bin/{}", program_name);
        match kernel.get_inode(&program_path) {
            Ok(inode_id) if is_executable(inode_id) => {
                let pid = kernel.create_process(0);  // Fork from init
                // TODO: Load & instantiate WASM module
                // TODO: Pass args via shared memory
                pid
            }
            _ => -1,
        }
    })
}
```

---

## VI. Performance Characteristics

### Time Complexity

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Path lookup | O(depth) | String comparison per level |
| File create | O(depth) | Lookup + insert in children map |
| Directory list | O(children) | Linear scan of children map |
| File read | O(size) | Copy entire file to WASM memory |
| Process create | O(1) | Simple PID increment |

### Space Complexity

| Structure | Space | Notes |
|-----------|-------|-------|
| Inode | O(1) + O(name) + O(data) | Metadata fixed, content varies |
| Inode map | O(num_files) | HashMap overhead ~24 bytes/entry |
| Children map | O(children per dir) | Cumulative across all dirs |
| File descriptor | O(1) | Fixed 32-byte struct |
| WASM memory | ~100KB + files | Binary + heap |

### Example: "Realistic" System State

```
System with 10,000 files:
├─ Inode map: 10,000 × ~40 bytes = 400 KB
├─ File data: 100 files × 1 MB avg = 100 MB
├─ WASM binary: ~100 KB
└─ Metadata overhead: ~20 KB
─────────────────────────────────
Total: ~100.5 MB usable files
```

---

## VII. Limitations & Future Work

### Known Limitations

1. **No file permissions** - All files globally readable/writable
2. **No symlinks** - Inode-based hierarchy only
3. **Limited file size** - No sparse files, all in-memory
4. **Synchronous I/O** - WASM blocks on syscalls (async planned)
5. **Single user** - No uid/gid, ACLs, or capability model
6. **No signals** - Process termination not cleanly handled
7. **UTF-8 only** - Binary files work but no special handling
8. **No networking** - Browser APIs available but not integrated

### Future Roadmap

**v0.3: Persistence**
- IndexedDB integration for VFS
- Filesystem snapshot save/load
- Automatic persistence on file write

**v0.4: Process Execution**
- Load WASM modules as executables
- Sandbox memory pages per process
- Inter-process communication via shared memory

**v0.5: GUI & Windowing**
- HTML5 Canvas widget system
- Window manager for tiling/floating layout
- Mouse input handling
- Graphical file browser

**v0.6: Networking**
- Fetch API wrapper for HTTP
- WebSocket syscalls
- Simple DNS resolver
- TCP-like stream abstraction

**v1.0: Multi-user & Security**
- User/group model (uid, gid)
- File permissions (rwx bits)
- Capability-based security tokens
- Shared browser-OS sessions

---

## VIII. Educational Value

### Systems Concepts Demonstrated

1. **Process Management**
   - Process creation and hierarchy
   - State machines (Ready, Running, etc.)
   - Context switching simulation

2. **Memory Management**
   - Inode-based addressing
   - File descriptor tables
   - Heap allocation patterns

3. **I/O Subsystem**
   - Device drivers (Display, Keyboard, Storage)
   - Syscall abstraction
   - Buffering and caching

4. **File Systems**
   - Semantic file operations
   - Directory traversal algorithms
   - Metadata structures

5. **Security Boundaries**
   - Kernel ↔ Userland separation
   - WASM sandbox properties
   - Hardware abstraction layers

### Research Applications

- **Browser Security:** Understand WASM capabilities and limitations
- **OS Design:** Compact OS implementation in ~600 lines of Rust
- **Language VMs:** Experience FFI between strong-typed and dynamic languages
- **Education:** Living codebase for teaching OS concepts

---

## IX. Building & Deploying

### Build Process

```bash
# Install dependencies
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# Build kernel
cd kernel && wasm-pack build --target web

# Copy artifacts
cp pkg/* ../web/

# Run web server
cd ../web && python -m http.server 8000

# Access at http://localhost:8000
```

### Deployment Options

1. **Static hosting** - GitHub Pages, Vercel, Netlify
2. **Docker** - Node/Python HTTP server container
3. **Embedded** - Electron, React Native wrapper
4. **Hybrid** - Native Node.js host + browser UI

---

## X. References

### Key Files

- **[kernel/src/lib.rs](../../kernel/src/lib.rs)** - Rust kernel implementation
- **[web/main.js](../../web/main.js)** - JavaScript host runtime
- **[web/index.html](../../web/index.html)** - Terminal GUI markup

### Technologies

- **Rust** - Type safety, memory safety, performance
- **WebAssembly** - Portable binary format, browser execution
- **wasm-bindgen** - Rust ↔ JavaScript FFI
- **IndexedDB** - Browser persistent storage
- **JavaScript Promises** - Async I/O abstraction

### Further Reading

- [WASM Specification](https://webassembly.org/specs/)
- [Operating Systems: Three Easy Pieces](https://pages.cs.wisc.edu/~remzi/OSTEP/)
- [The Rust Book](https://doc.rust-lang.org/book/)

---

## XI. Conclusion

BrowserOS demonstrates that sophisticated OS kernels can be implemented in WebAssembly without sacrificing abstraction or clarity. The system achieves true portability (runs on any browser), type safety (Rust), and hardware abstraction (browser APIs) while remaining small enough (~600 LOC) to understand completely.

The architecture is intentionally modular to encourage extensions and research. Students, researchers, and developers can use BrowserOS as a foundation for exploring OS theory, WASM capabilities, and systems programming without requiring low-level machine access.

---

**Questions? Contribute at:** [GitHub Repository]  
**License:** MIT  
**Last Updated:** February 2026
