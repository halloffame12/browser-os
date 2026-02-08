# BrowserOS - A Virtual Operating System in WebAssembly

A research-level operating system kernel written in Rust and compiled to WebAssembly (WASM), running entirely inside a web browser. BrowserOS demonstrates core OS concepts: process management, virtual file systems, syscall abstraction, and cooperative multitasking.

```
┌─────────────────────────────────────────────┐
│         Your Browser (Chrome, Firefox, etc) │
│  ┌─────────────────────────────────────────┐│
│  │  BrowserOS Shell (Terminal Interface)   ││
│  │  $ ls /bin                              ││
│  │  cat  ls  echo  touch  mkdir            ││
│  │  $ cat /etc/config.sys                  ││
│  │  BrowserOS Configuration File           ││
│  └─────────────────────────────────────────┘│
│                                              │
│  WASM Kernel (Rust) + JS Drivers            │
│  ├─ Process Manager    ├─ Display Driver    │
│  ├─ File System        ├─ Keyboard Driver   │
│  ├─ Syscall Dispatcher ├─ Storage Driver    │
│  └─ I/O Subsystem      └─ Timer             │
└─────────────────────────────────────────────┘
```

## Quick Start (30 seconds)

### Prerequisites
- Rust 1.70+ with WASM target: `rustup target add wasm32-unknown-unknown`
- wasm-pack: `cargo install wasm-pack`
- Python 3 (for HTTP server)

### Build & Run

```bash
# Build Rust kernel to WASM
cd kernel && wasm-pack build --target web

# Copy artifacts
cp pkg/* ../web/

# Start web server
cd ../web && python -m http.server 8000

# Open browser: http://localhost:8000
```

### Try These Commands

```
help          # Show available commands
uname         # Display OS info
uptime        # Show system uptime
ls /          # List root directory
mkdir /tmp    # Create directory
touch /tmp/file.txt
cd /tmp       # Change directory
pwd           # Show current path
cat /tmp/file.txt
echo hello > /tmp/msg.txt
ps            # List processes
clear         # Clear terminal
```

---

## Tasks & Example Workflows

### Task 1: Create a Project Directory

```bash
# Create a new project
$ mkdir /home/projects
$ cd /home/projects
$ pwd
/home/projects

# Verify directory exists
$ ls /home
projects

# Create subdirectories
$ mkdir /home/projects/myapp
$ mkdir /home/projects/myapp/src
$ mkdir /home/projects/myapp/bin
```

### Task 2: Create and Edit Files

```bash
# Create a README file with content
$ echo "My Awesome Project" > /home/projects/myapp/README.md
$ echo "Written in Rust and WASM" >> /home/projects/myapp/README.md

# Read the file
$ cat /home/projects/myapp/README.md
My Awesome Project
Written in Rust and WASM

# Create a source file
$ echo "fn main() { println!(\"Hello\"); }" > /home/projects/myapp/src/main.rs
$ cat /home/projects/myapp/src/main.rs
fn main() { println!("Hello"); }
```

### Task 3: Organize Files

```bash
# Create a documents folder
$ mkdir /home/documents
$ cd /home/documents

# Create multiple files
$ echo "Meeting notes - 2026-02-08" > notes.txt
$ echo "TODO List:" > tasks.txt
$ echo "1. Learn BrowserOS" >> tasks.txt
$ echo "2. Write WASM code" >> tasks.txt
$ echo "3. Build OS kernel" >> tasks.txt

# View all files
$ ls /home/documents

# Read task list
$ cat /home/documents/tasks.txt
TODO List:
1. Learn BrowserOS
2. Write WASM code
3. Build OS kernel
```

### Task 4: System Information & Status

```bash
# Get system information
$ uname
BrowserOS v0.2 (WASM Research Edition)

# Check system uptime
$ uptime
Uptime: 0h 0m 45s

# List running processes
$ ps
PID 0: init (running)

# Check current location
$ pwd
/home/documents
```

### Task 5: Navigate Complex Directory Trees

```bash
# Create a multi-level directory structure
$ mkdir /home/users
$ mkdir /home/users/alice
$ mkdir /home/users/alice/documents
$ mkdir /home/users/alice/documents/projects
$ mkdir /home/users/alice/documents/projects/web
$ mkdir /home/users/alice/documents/projects/mobile

# Navigate the tree
$ cd /home/users/alice/documents/projects/web
$ pwd
/home/users/alice/documents/projects/web

# Go up and list contents
$ cd /home/users/alice/documents
$ ls /home/users/alice/documents/projects
web,mobile

# List deeply nested folder
$ ls /home/users/alice
documents
```

### Task 6: Backup & Archive (Concept)

```bash
# Create original file
$ echo "Important configuration data" > /etc/config.sys
$ cat /etc/config.sys
Important configuration data

# Create backup location
$ mkdir /tmp/backups
$ echo "Important configuration data" > /tmp/backups/config.sys.bak

# Verify both exist
$ cat /etc/config.sys
Important configuration data
$ cat /tmp/backups/config.sys.bak
Important configuration data
```

### Task 7: Create a Website Structure

```bash
# Design a basic site structure
$ mkdir /home/website
$ mkdir /home/website/public
$ mkdir /home/website/public/css
$ mkdir /home/website/public/js
$ mkdir /home/website/public/images

# Create files
$ echo "body { color: green; }" > /home/website/public/css/style.css
$ echo "console.log('Hello from JS');" > /home/website/public/js/app.js
$ echo "<h1>Welcome to BrowserOS</h1>" > /home/website/public/index.html

# Navigate and view
$ cd /home/website/public
$ pwd
/home/website/public
$ ls /home/website/public
css,js,images,index.html
```

### Task 8: System Maintenance Simulation

```bash
# Create log files
$ mkdir /var/logs
$ echo "System boot at 2026-02-08" > /var/logs/boot.log
$ echo "Kernel version 0.2" >> /var/logs/boot.log

# Create error log
$ echo "No errors during startup" > /var/logs/error.log

# Create process list snapshot
$ ps > /var/logs/ps.snap
$ cat /var/logs/ps.snap
PID 0: init (running)

# View all logs
$ ls /var/logs
boot.log,error.log,ps.snap
```

### Task 9: Text File Processing

```bash
# Create a data file
$ echo "name:age:location" > /tmp/data.txt
$ echo "Alice:25:NYC" >> /tmp/data.txt
$ echo "Bob:30:LA" >> /tmp/data.txt
$ echo "Charlie:28:Chicago" >> /tmp/data.txt

# Read back the file
$ cat /tmp/data.txt
name:age:location
Alice:25:NYC
Bob:30:LA
Charlie:28:Chicago

# Create a processed copy
$ echo "CSV Data" > /tmp/data.csv
$ echo "Alice, 25, NYC" >> /tmp/data.csv
$ echo "Bob, 30, LA" >> /tmp/data.csv

$ cat /tmp/data.csv
CSV Data
Alice, 25, NYC
Bob, 30, LA
```

### Task 10: Project Bootstrap

```bash
# Create a new project from scratch
$ mkdir /home/myproject
$ mkdir /home/myproject/{src,bin,docs,tests}
$ mkdir /home/myproject/src/{main,lib}

# Create manifest
$ echo "[project]" > /home/myproject/Cargo.toml
$ echo "name = \"myproject\"" >> /home/myproject/Cargo.toml
$ echo "version = \"0.1.0\"" >> /home/myproject/Cargo.toml

# Create source files
$ echo "// Main entry point" > /home/myproject/src/main/main.rs
$ echo "// Library code" > /home/myproject/src/lib/mod.rs

# Create documentation
$ echo "# MyProject" > /home/myproject/docs/README.md
$ echo "A cool Rust project" >> /home/myproject/docs/README.md

# Create test file
$ echo "// Tests go here" > /home/myproject/tests/integration_test.rs

# Verify structure
$ ls /home/myproject
src,bin,docs,tests,Cargo.toml
$ ls /home/myproject/docs
README.md
```

---

## Architecture Overview

### System Stack

```
Browser Runtime (JavaScript)
         ↓
[Display] [Keyboard] [Storage] [Timer]  ← Device Drivers
         ↓
  WASM ↔ JavaScript Bridges (wasm-bindgen)
         ↓
    Rust Kernel (WASM)
    ├─ Process Manager
    ├─ Virtual File System (VFS)
    ├─ Syscall Dispatcher
    └─ I/O Abstraction
         ↓
  JavaScript Shell Interface
```

### Core Components

| Component | Language | Purpose |
|-----------|----------|---------|
| **Kernel** | Rust | Process management, VFS, syscalls |
| **WASM Binary** | WebAssembly | Portable binary target |
| **Host Runtime** | JavaScript | Device drivers, system services |
| **Shell** | JavaScript | Command parsing, user interface |
| **Terminal** | HTML/CSS | Visual output display |

---

## File Structure

```
browser-os/
├── kernel/
│   ├── Cargo.toml          # Rust project config
│   ├── src/
│   │   └── lib.rs          # Kernel implementation (~500 LOC)
│   └── pkg/                # Build artifacts (generated)
│       ├── browser_os.js       # WASM bindings
│       └── browser_os_bg.wasm  # Compiled kernel
├── web/
│   ├── index.html          # Terminal UI
│   ├── main.js             # JS host runtime (~400 LOC)
│   ├── browser_os.js       # WASM module (generated)
│   └── browser_os_bg.wasm  # WASM binary (generated)
├── ARCHITECTURE.md         # Detailed design doc
└── README.md               # This file
```

---

## Design Decisions & Why

### Browser as Hardware

| OS Concept | Hardware Role | Browser Implementation |
|-----------|---|---|
| CPU | Instruction execution | JavaScript/WASM engine |
| RAM | Memory storage | WASM linear memory |
| Disk | File storage | IndexedDB, localStorage |
| Display | Screen output | DOM + Canvas |
| Keyboard | Input device | Event listeners |
| Clock | System time | Date API, requestAnimationFrame |

**Why?** The browser provides a complete, portable hardware abstraction. The kernel runs unchanged on any operating system with a modern browser.

### Rust for the Kernel

**Pros:**
- ✅ Type safety → fewer bugs
- ✅ Memory safety → no buffer overflows
- ✅ Zero-cost abstractions → small WASM binary
- ✅ Excellent WASM ecosystem

**Cons:**
- ❌ Longer compile times
- ❌ Steeper learning curve
- ❌ No runtime garbage collection

### Virtual File System Design

**Inode-based architecture:**
- File = inode (metadata) + data (content)
- Directory = special inode containing child name→inode mappings
- Path traversal: `/home/user/file.txt` → lookup each component in parent

**In-memory storage (v0.2):**
- Files stored in WASM heap (Vec<u8>)
- No persistence across page reloads
- Fast performance (microsecond access)
- Future: IndexedDB integration for persistence

### Process Model

**Cooperative multitasking:**
- WASM is single-threaded (no preemption possible)
- Processes yield control voluntarily
- Simpler than preemptive scheduling
- Sufficient for educational purposes

**Process lifecycle:**
```
create → ready → running → [waiting] → terminated
```

---

## Implemented Commands

### File System

```bash
ls [path]       # List directory contents
cat [path]      # Print file contents  (fs_cat)
touch [path]    # Create empty file    (fs_create)
mkdir [path]    # Create directory     (fs_create)
```

### Navigation

```bash
pwd             # Print working directory
cd [path]       # Change working directory
```

### System

```bash
help            # Show command reference
uname           # Display OS name/version
uptime          # Show system uptime (tracked by kernel)
ps              # List processes        (init process shown)
clear           # Clear terminal screen
```

### Advanced

```bash
echo TEXT > FILE        # Redirect output to file
cat FILE                # Read file with output redirection
```

### Not Yet Implemented

```bash
pipe (|)        # Connect program outputs
env             # Environment variables
export VAR=val  # Set variables
for/while       # Control flow
```

---

## Syscall Interface

The kernel exposes syscalls via `wasm-bindgen`:

### File System Syscalls

```rust
fs_create(path: &str, is_dir: bool) -> i32
fs_open(path: &str, mode: &str) -> i32
fs_read(fd: u32, size: usize) -> String
fs_write(fd: u32, data: &str) -> i32
fs_close(fd: u32) -> i32
fs_list(path: &str) -> String        // List directory
fs_cat(path: &str) -> String         // Read file content
```

### Process Syscalls

```rust
process_spawn(parent_pid: u32) -> u32   // Create process
get_uptime() -> u64                     // Get system uptime (ms)
```

### System Syscalls

```rust
boot(current_time_ms: u64) -> String    // Initialize kernel
update_time(current_time_ms: u64)       // Sync system clock
uname() -> String                       // System info
handle_command(cmd: &str) -> String     // Legacy interface
```

---

## Virtual File System Layout

```
/                           # Root directory (inode 0)
├── bin/                    # User programs
│   ├── sh                  # Shell (would be WASM module)
│   ├── cat                 # Cat utility
│   ├── ls                  # List utility
│   └── echo                # Echo utility
├── etc/                    # System configuration
│   └── config.sys          # System config file
├── home/                   # User home directories
│   └── user/               # User home
│       └── documents/
└── tmp/                    # Temporary files
```

**Capabilities:**
- ✅ Create/read/write files
- ✅ Create/list/delete directories
- ✅ Hierarchical paths with `/` separator
- ❌ File permissions (all files open)
- ❌ Symlinks/hardlinks
- ❌ Sparse files

---

## Extending BrowserOS

### Add a New Command

**Example: `date` command**

Edit `web/main.js` → `Shell.execute()`:

```javascript
case "date":
    output = new Date().toString();
    break;
```

### Add a New Syscall

**Example: `get_random()` syscall**

**Step 1: Rust `kernel/src/lib.rs`**
```rust
#[wasm_bindgen]
pub fn sys_random() -> u32 {
    (Math.random() * u32::MAX as f64) as u32
}
```

**Step 2: JavaScript `web/main.js`**
```javascript
case "random":
    output = kernel.sys_random().toString();
    break;
```

### Add Persistence

**Enable IndexedDB for VFS:**

```javascript
// In main() after boot:
const savedState = await storage.load("vfs_snapshot");
if (savedState) {
    kernel.restore_vfs(savedState);
}

// On file write:
await storage.save("vfs_snapshot", kernel.serialize_vfs());
```

### Implement Process Execution

**Load WASM modules as executables:**

```rust
#[wasm_bindgen]
pub fn exec(program_path: &str, args: &str) -> u32 {
    // 1. Read program from filesystem
    // 2. Load WASM module
    // 3. Create new process
    // 4. Pass args via shared memory
    // 5. Return PID
}
```

---

## Performance & Limitations

### Performance Characteristics

| Operation | Time Complexity | Notes |
|-----------|---|---|
| Path lookup | O(depth) | String comparison per directory level |
| File read | O(size) | Copy entire file content |
| Directory list | O(children) | Linear scan of directory entries |
| File write | O(size) | Append to vector |
| Process creation | O(1) | Simple PID allocation |

### Limitations (v0.2)

- ❌ **No file permissions** - All files globally readable/writable
- ❌ **No persistence** - Files lost on page reload
- ❌ **No pipes** - `|` operator not implemented
- ❌ **Single user** - No uid/gid system
- ❌ **No networking** - Could add via Fetch API
- ⚠️ **UTF-8 only** - Binary files in-memory only

### Resource Usage

```
Typical system state:
├─ WASM binary: ~100 KB
├─ Kernel heap: ~50 KB (data structures)
├─ File storage: Variable (in memory)
└─ Total: ~200 KB + files

Browser support:
├─ Storage quota: 50+ MB (IndexedDB)
├─ Memory limit: System RAM
└─ Compatible: Chrome, Firefox, Safari, Edge (2021+)
```

---

## Building from Source

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack

# Verify Python (for HTTP server)
python --version
```

### Build Steps

```bash
# Change to kernel directory
cd browser-os/kernel

# Build Rust → WASM (produces pkg/*)
wasm-pack build --target web --release

# Copy artifacts to web dir
cp -r pkg/* ../web/

# Start development server
cd ../web
python -m http.server 8000

# Open http://localhost:8000 in browser
```

### Rebuild After Changes

```bash
# Modify kernel code
vim kernel/src/lib.rs

# Rebuild
cd kernel && wasm-pack build --target web && cp pkg/* ../web/

# Browser will auto-reload (if using watch mode)
```

---

## Testing

### Manual Testing

```bash
$ help
=== BrowserOS Shell ===
FILESYSTEM COMMANDS:
  ls [path]          - List directory contents
  cat [path]         - Print file contents
  ...

$ ls /
bin,etc,home,tmp

$ mkdir /test && cd /test && pwd
/test

$ echo "Hello World" > msg.txt && cat msg.txt
Hello World

$ uptime
Uptime: 0h 0m 12s
```

### Automated Testing (Future)

```bash
# Integration tests via Playwright
npm install --save-dev @playwright/test

# Example test:
test('creates file and reads it', async () => {
    await page.type("#input", "touch /tmp/test.txt");
    await page.press("#input", "Enter");
    await page.type("#input", "cat /tmp/test.txt");
    await page.press("#input", "Enter");
    // Assert output...
});
```

---

## Security Considerations

### Attack Surface

1. **WASM Sandbox** ✅ Protected
   - WASM memory isolated from browser
   - No direct system calls allowed
   - Kernel can validate all operations

2. **JavaScript →  WASM** ⚠️ Kernel vulnerability only
   - Malicious JS could pass invalid syscalls
   - Kernel validates all inputs (bounds-checking, path validation)

3. **Browser APIs** ✅ Protected
   - IndexedDB sandboxed per origin
   - localStorage isolated by domain
   - Same-origin policy enforced

### Best Practices NOT Yet Implemented

- ❌ Input validation (paths not checked for directory traversal)
- ❌ Resource limits (file size, number of processes, open files)
- ❌ Access control (no file permissions or user model)
- ❌ Audit logging (no syscall trace)

---

## Booting Sequence (v0.2)

```
Page load
   ├──→ Load HTML/CSS/JS
   │     ├─ Display: Initialize terminal element
   │     ├─ Keyboard: Register input listener  
   │     └─ Storage: Open IndexedDB connection
   │
   ├──→ Fetch WASM binary (browser_os_bg.wasm)
   │
   ├──→ Instantiate WASM module
   │     └─ Link imports (Math.random, etc)
   │
   ├──→ Call kernel.boot()
   │     ├─ Initialize process table (PID 0 = init)
   │     ├─ Create root filesystem (/)
   │     ├─ Initialize inode map (inode 0 = root)
   │     ├─ Create directories: /bin, /etc, /home, /tmp
   │     └─ Return boot message
   │
   ├──→ Display boot message
   │
   ├──→ Show prompt
   │
   └──→ Ready for input (kernel.handle_command loop)
```

---

## Understanding the Code

### Rust Kernel (kernel/src/lib.rs)

```rust
// Main concepts:
1. KERNEL: thread_local!{ static KERNEL: RefCell<Kernel> }
   └─ Global mutable kernel state
   
2. Kernel struct contains:
   ├─ booted: bool                          # Boot flag
   ├─ process_table: HashMap<u32, PCB>     # PID → Process
   ├─ inode_map: HashMap<u32, Inode>       # Inode# → File/Dir
   └─ open_files: HashMap<u32, FileDesc>   # FD → File metadata
   
3. Syscall pattern:
   #[wasm_bindgen] pub fn fs_open(path) -> i32 {
       KERNEL.with(|k| {
           let mut kernel = k.borrow_mut();  # Mutable borrow
           // Syscall implementation
       })
   }
```

### JavaScript Runtime (web/main.js)

```javascript
// Main concepts:
1. Device Drivers
   ├─ Display: write() / clear()
   ├─ Keyboard: onInput() / emit()
   └─ Storage: save() / load()
   
2. Shell class
   └─ execute(cmdLine) → matches command, calls kernel
   
3. WASM integration
   ├─ await import("./browser_os.js") → module
   ├─ kernel.fs_cat(path) → syscall
   └─ display.write(output) → show result
```

---

## Troubleshooting

### WASM Module Not Loading

**Error:** "Failed to instantiate module"

**Solution:**
1. Check browser console (F12 → Console tab)
2. Verify `browser_os_bg.wasm` exists in web/ directory
3. Rebuild: `cd kernel && wasm-pack build --target web && cp pkg/* ../web/`
4. Hard-refresh browser (Ctrl+Shift+R)

### Commands Not Working

**Error:** "syscall returned -1"

**Solution:**
1. Check path format: absolute paths start with `/`
2. Verify directory exists before creating files in it
3. Create parent directories first: `mkdir /tmp` then `touch /tmp/file.txt`

### File Content Not Saving

**Issue:** Files created but content saved as empty

**Likely cause:** File doesn't yet support write-after-create  

**Workaround:** Use output redirection instead:
```bash
echo "Hello" > /tmp/file.txt      # Works
(vs)
touch /tmp/file.txt && echo? "Hello" >> /tmp/file.txt   # Not yet supported
```

---

## Contributing

### Contribution Ideas

- [ ] Implement `cat [path]` with file write support
- [ ] Add `grep` command
- [ ] Implement output piping (`|` operator)  
- [ ] Add environment variables and `export`
- [ ] IndexedDB persistence
- [ ] File permissions (rwx bits)
- [ ] Implement `su` / user switching
- [ ] Add `man` pages for commands
- [ ] GUI file manager
- [ ] Network filesystem driver

### Development Workflow

```bash
# 1. Fork & clone
git clone https://github.com/yourusername/browser-os
cd browser-os

# 2. Make changes
vim kernel/src/lib.rs                # Edit kernel
vim web/main.js                      # Edit shell

# 3. Test changes
cd kernel && wasm-pack build --target web
cp pkg/* ../web/ && cd ../web && python -m http.server 8000

# 4. Open http://localhost:8000 and test manually
# F12 → Console for errors

# 5. Commit & push
git add .
git commit -m "Add feature: xyz"
git push origin feature/xyz

# 6. Create pull request on GitHub
```

---

## License

MIT License - See LICENSE file for details

---

## Acknowledgments

- Rust WASM book & community
- Operating Systems: Three Easy Pieces (OSTEP)
- WebAssembly research and standards bodies

---

## Questions?

- 📖 Read [ARCHITECTURE.md](./ARCHITECTURE.md) for deep dives
- 🐛 Open an issue on GitHub
- 💬 Discussions section for questions
- 🔬 Research applications? Email maintainer

---

**Let's explore OS design in the browser! 🚀**

Built with: Rust 🦀 | WebAssembly ⚙️ | JavaScript 📱 | Love ❤️
