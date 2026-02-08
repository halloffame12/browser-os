# BrowserOS Design Decisions & Tradeoffs

This document details the rationale behind major architectural decisions made in BrowserOS and discusses alternatives that were considered.

---

## 1. Kernel Language: Rust vs Alternatives

### Decision: Rust for the kernel

### Rationale

| Criterion | Rust | C | TypeScript | Go |
|-----------|------|---|-----------|-----|
| Type Safety | ✅ Strong | ❌ Weak | ✅ Strong | ✅ Strong |
| Memory Safety | ✅ Compile-time | ❌ Runtime | ✅ Automatic GC | ⚠️ Partial |
| WASM Support | ✅ Excellent | ✅ Good | ⚠️ Via TSVM | ✅ Good |
| Binary Size | ✅ ~100KB | ✅ ~80KB | ❌ ~500KB | ❌ ~200KB |
| Build Time | ❌ Slow | ✅ Fast | ✅ Fast | ✅ Fast |
| Learning Curve | ❌ Steep | ⚠️ Medium | ✅ Easy | ✅ Easy |

### Tradeoffs Accepted

**Disadvantages of Rust:**
1. **Compilation time**: ~30 seconds for initial build, ~1 second for incremental
   - Mitigated by: Cargo caching, wasm-opt in release mode
2. **Steep learning curve**: Borrow checker requires careful ownership thinking
   - Mitigated by: Excellent compiler error messages
3. **Verbose syntax**: More code than Python or JS
   - Mitigated by: Macros reduce boilerplate (e.g., derives)

**Advantages that justified the cost:**
- Zero runtime errors from memory safety → confidence in production use
- Excellent error messages guide learning
- WASM-friendly (wasm-bindgen is best-in-class)
- Small binary size (100KB vs 500KB with TS)

### Alternatives Considered

#### TypeScript/Node.js Kernel
```typescript
// Would look like:
class Kernel {
    fileSystem = new Map<string, Inode>()
    processes = new Map<number, PCB>()
    
    openFile(path: string) {
        // Would require bounds checking everywhere
        const inode = this.fileSystem.get(path)
        if (!inode) throw new Error(...)
    }
}
```
**Rejected because:**
- WASM binary would be 5× larger (~500KB)
- Less optimal memory layout
- Slower execution (GC pauses)
- Type safety only at compile-time (can be disabled at runtime)

#### C Kernel Compiled to WASM
```c
// Would look like:
int open_file(const char* path, char mode) {
    FileDescriptor* fd = malloc(sizeof(FileDescriptor));
    if (!fd) return -1;
    strcpy(fd->path, path);  // Buffer overflow risk!
    // ...
}
```
**Rejected because:**
- Manual memory management error-prone
- No automatic bounds-checking
- Potential security issues (buffer overflows)
- Code would be harder to audit and maintain

#### Go Kernel
```go
func (k *Kernel) OpenFile(path string) (int, error) {
    // Similar to Rust but:
    // - Larger binary (~200KB)
    // - Slower GC pauses
    // - Less control over memory layout
}
```
**Rejected because:**
- Similar memory safety as Rust but less control
- Heavier runtime
- WASM support is newer and less optimized

---

## 2. Kernel Architecture: Monolithic vs Microkernel

### Decision: Monolithic kernel (single WASM address space)

### Rationale

```
MONOLITHIC DESIGN (chosen):
┌─────────────────────────┐
│  WASM Kernel Process    │
│  • Process Manager      │
│  • File System          │
│  • Syscall Dispatcher   │
│  • Memory Management    │
│  • (All in one process) │
└─────────────────────────┘
```

**Advantages:**
- Simpler implementation (~500 LOC vs 2000+ for microkernel)
- No inter-process communication overhead
- Easier debugging (single address space)
- Natural in WASM environment

**Disadvantages mitigated by:**
- Kernel isolation from userland via WASM boundaries
- JavaScript acts as privilege separation layer
- No actual separation needed (research OS)

### Alternatives Considered

#### Microkernel Architecture
```
┌──────────────────────────────┐
│  HAL (Hardware Abstraction)  │ ← Minimal kernel in WASM
├───────────┬──────────┬───────┤
│ FileServer │ ProcSvr │ IPC   │ ← Services in separate modules
└───────────┴──────────┴───────┘
```

**Rejected because:**
- Significantly more complex (RPC, message passing)
- Higher latency between components
- Harder to reason about in educational context
- WASM doesn't naturally support multiple address spaces
- Would require complex service composition

#### Exokernel (Resource Allocation Only)
```
┌─────────────────────────┐
│  Exokernel              │
│  (Just allocate memory) │
└────────────────────────┐
│ Userland FS, Proc, IPC │
```

**Rejected because:**
- Requires userland OS code (infeasible for WASM environment)
- No actual hardware to virtualize
- Adds complexity without benefit

---

## 3. File System: In-Memory vs Persistent Storage

### Decision: In-memory for v0.2, IndexedDB for v0.3+

### Current Design (v0.2): Pure In-Memory

```rust
pub struct Inode {
    data: Vec<u8>,  // ← Stored in WASM linear memory
}

inode_map: HashMap<u32, Inode>  // ← Heap-allocated
```

**Advantages:**
- Zero I/O latency
- Simplest implementation
- No serialization overhead
- Easy to debug

**Disadvantages:**
- Lost on page reload
- Limited by available RAM (~50MB typical)

### Future Design (v0.3): IndexedDB Integration

```rust
// Kernel would support:
pub fn serialize_vfs() -> String { ... }
pub fn deserialize_vfs(json: &str) { ... }

// JavaScript would coordinate:
async function boot() {
    const state = await storage.load("vfs_snapshot")
    kernel.deserialize_vfs(state)
}

window.addEventListener("beforeunload", async () => {
    const state = kernel.serialize_vfs()
    await storage.save("vfs_snapshot", state)
})
```

**Why IndexedDB over others:**
1. **LocalStorage**: 
   - ✅ Simpler API
   - ❌ Limited to 5-10MB
   - ❌ Synchronous (blocks thread)
   
2. **IndexedDB**:
   - ✅ 50MB+ quota
   - ✅ Async APIs (non-blocking)
   - ✅ Better for large files
   - ❌ More complex API
   
3. **Cache API** / **FileSystem API**:
   - ⚠️ More experimental
   - ⚠️ Limited browser support
   - ⚠️ Restricted scope

### Alternatives Considered

#### Blockchain/IPFS Persistence
```
BrowserOS → IPFS [Distributed Filesystem]
```
**Rejected because:**
- Adds massive external dependency
- Unnecessary for local research OS
- Introduces network latency
- Complicates booting

#### Cloud Storage (S3/Firebase)
```
BrowserOS → Firebase Realtime Database
```
**Rejected because:**
- Requires authentication
- Introduces privacy concerns
- Network-dependent (no offline mode)
- Out of scope for browser research

---

## 4. Process Model: Cooperative vs Preemptive Multitasking

### Decision: Cooperative multitasking (v0.2), Async simulator for v0.3+

### Rationale

```
COOPERATIVE (chosen, v0.2):
┌──────────────────────────────┐
│  Process A yields            │
├──────────────────────────────┤
│  Scheduler runs Process B    │
├──────────────────────────────┤
│  Process B yields            │
├──────────────────────────────┤
│  Back to Process A           │
└──────────────────────────────┘

Reality in WASM:
• Single thread executes WASM code
• Scheduler runs in JavaScript event loop
• No true parallelism possible
```

**Advantages:**
- Simpler to implement (no interrupt handlers)
- No race conditions (single-threaded)
- Deterministic execution (easier testing)

**Disadvantages mitigated by:**
- Not relevant for single-threaded WASM
- Educational purposes only (no real multi-core)

### Alternatives Considered

#### Preemptive Scheduling
```rust
// Would require:
struct InterruptHandler {
    timer_interval_ms: u32,
    save_cpu_state: fn(),
    restore_cpu_state: fn(),
}

// JavaScript would need:
setInterval(() => {
    kernel.trigger_context_switch()  // Force yield
}, 10)  // Every 10ms
```

**Rejected because:**
- WASM has no interrupt mechanism
- Would require simulating register save/restore
- No benefit in single-threaded environment
- Added complexity for research OS

#### Async/Await Simulation
```javascript
// Would look like:
async function executeProgram(pid) {
    while (process.state != "terminated") {
        await kernel.execute_timeslice(pid, 10)  // 10ms slice
        await new Promise(r => setTimeout(r, 0))  // Yield
    }
}
```

**Deferred to v0.3** because:
- More complex than current needs
- Would require refactoring much code
- Environmental processes don't need timer-based preemption
- Can be layered on top without kernel changes

---

## 5. Syscall Interface: RPC Style vs High-Level Operations

### Decision: High-level syscall operations (fs_cat, fs_open, etc)

### Rationale

```
CHOSEN APPROACH: High-level syscalls
┌──────────────────────────┐
│ JS: fs_cat("/tmp/file")  │
├──────────────────────────┤
│ Kernel:                  │
│  1. Lookup inode         │
│  2. Read content         │
│  3. Return string        │
└──────────────────────────┘
Line count: 1 (JS) + 10 (Rust)
```

**Advantages:**
- Simple interface for JavaScript
- Easy to use in shell
- Minimal WASM ↔ JS transitions
- Clear semantics

**Disadvantages systematically addressed by:**
- None severe; interface is appropriate for research OS

### Alternative: Unix-like Low-Level Syscalls

```rust
// Would look like:
const O_RDONLY = 0
const O_WRONLY = 1
const O_CREAT = 0x40

#[wasm_bindgen]
pub fn open(pathname: &str, flags: u32) -> i32
pub fn read(fd: u32, buf_ptr: u32, len: u32) -> i32
pub fn write(fd: u32, buf_ptr: u32, len: u32) -> i32
pub fn close(fd: u32) -> i32
pub fn stat(pathname: &str, buf_ptr: u32) -> i32
```

Benefits of Unix approach:
- ✅ More flexible (composable operations)
- ✅ Better for lower-level languages (C, Rust)
- ✅ Actual POSIX compatibility

Drawbacks:
- ❌ More complex for JavaScript shell
- ❌ More memory management (buffers)
- ❌ Requires shared memory for stat structures
- ❌ Higher WASM ↔ JS overhead (per syscall)

**Decision rationale:** High-level syscalls sufficient for v0.2 shell. Can add Unix-style syscalls in v1.0 when implementing userspace programs.

---

## 6. Filesystem Representation: Hierarchical vs Flat

### Decision: Hierarchical (standard Unix-like paths)

### Rationale

```
CHOSEN: Hierarchical
/home/user/documents/report.txt
 ↓
 Start at root inode
   ↓ lookup "home"
     ↓ lookup "user"
       ↓ lookup "documents"
         ↓ lookup "report.txt" file
           ↓ read inode 4782 content
```

**Advantages:**
- Familiar to all users
- Natural organization
- Scales to many files
- Supports permission hierarchies (future)

### Alternative: Flat Namespace

```
Report file stored as: "home.user.documents.report.txt"

Disadvantages:
- ✗ Harder to parse (string splitting)
- ✗ Less intuitive for users
- ✗ Difficult to implement directory operations
- ✗ No natural permission inheritance
```

**Clearly wrong choice from UX perspective.**

### Alternative: Tagged/Attributed Filesystem

```
FileSystem could use:
{
    "report.txt": {
        "path": ["/", "home", "user", "documents"],
        "owner": "user",
        "created": 1708900000,
        "content": "..."
    }
}
```

**Not chosen because:**
- More complex representation
- Harder to enforce uniqueness
- Inefficient queries ("find all files in /home")

---

## 7. String Encoding: UTF-8 vs Custom Binary Protocol

### Decision: UTF-8 strings for simplicity

### Current Implementation

```rust
// Syscall returns data as comma-separated bytes:
#[wasm_bindgen]
pub fn fs_read(fd: u32, size: usize) -> String {
    data.iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(",")  // "72,101,108,108,111" for "Hello"
}

// JavaScript decodes:
const bytes = dataString.split(",").map(Number)
const text = new TextDecoder().decode(new Uint8Array(bytes))
```

**Advantages:**
- No binary encoding complexity
- Easy debugging (can read syscall args)
- Works with JavaScript strings

**Disadvantages mitigated:**
- Performance: Extra string allocations
  - Acceptable: File I/O typically dominates
  - Potential optimization: Use shared memory for large transfers

### Alternative: Shared Memory (WasmMem)

```rust
// More efficient but complex:
pub struct WasmMemory {
    buffer: Box<[u8]>,  // Shared with JS
    len: usize,
}

#[wasm_bindgen]
pub fn fs_read_into_buffer(fd: u32) -> u32 {
    // Reads directly into shared buffer
    // Returns byte count
}
```

**Not chosen for v0.2 because:**
- Adds manual memory management
- Harder to debug
- Overkill for current file sizes
- Would break wasm-bindgen type system

**Deferred to v0.3** for binary file support.

---

## 8. Process Representation: Thin vs Rich PCB

### Decision: Minimal Process Control Block

### Chosen Design

```rust
pub struct ProcessControlBlock {
    pid: u32,
    state: ProcessState,
    parent_pid: Option<u32>,
    exit_code: i32,
}
// = 20 bytes per process
```

**Advantages:**
- Memory efficient (1,000 processes = 20KB)
- Simple to reason about
- Sufficient for educational purposes

**Disadvantages mitigated:**
- No register file → acceptable (WASM handles registers)
- No memory map → acceptable (no virtual memory)
- No signal handlers → acceptable (no signals yet)

### Rich PCB Alternative

```rust
pub struct ProcessControlBlock {
    pid: u32,
    state: ProcessState,
    parent_pid: Option<u32>,
    children_pids: Vec<u32>,
    memory_map: MemoryMap,
    open_files: [Option<u32>; 256],
    signal_handlers: [fn(); 64],
    exit_code: i32,
    rusage: ResourceUsage,
}
// = 2000+ bytes per process
```

**When to use (OS 301 course):**
- Teaching about resource limits
- Signal handling
- Process resource tracking
- Scheduler complexity

---

## 9. Interface Style: Terminal vs GUI

### Decision: Terminal first (v0.2), GUI framework prepared for v0.3

### Rationale

```
CHOSEN: Terminal Interface
┌────────────────────────────────┐
│ $ ls /home                    │
│ user documents download        │
│ $ cat documents/note.txt      │
│ Remember to refactor!          │
│ $ _                            │
└────────────────────────────────┘
Lines of code: ~400 JS (already written)
```

**Advantages:**
- Familiar to developers
- Easy to implement
- POSIX compatibility
- Scriptable

**Disadvantages mitigated:**
- Less visual → acceptable for research
- Not as intuitive for end-users → acceptable (dev-focused)

### GUI Alternative

```javascript
// Would require:
class WindowManager {
    createWindow(title, width, height)
    drawRect(x, y, w, h, color)
    handleMouseClick(x, y)
}

class Canvas {
    drawFileManager()
    drawTextEditor()
    drawDesktop()
}

// Roughly 1000+ LOC
```

**Deferred to v0.4** because:
- Adds 2× the code
- Not essential for OS functionality
- HTML Canvas complex to debug
- Terminal sufficient for research

### Web UI Alternative

```html
<!-- REST-API based -->
<button onclick="kernel.rm_recursive('/tmp')">Delete</button>
<input type="file" onchange="upload(file)">
<div id="fileTree">...</div>
```

**Not chosen because:**
- Requires rewriting kernel API in REST style
- More brittle (HTTP has higher overhead)
- Harder to debug
- Not better UX than terminal

---

## 10. Build System: wasm-pack vs Custom Build

### Decision: wasm-pack + Cargo

### Rationale

```bash
# One command builds everything:
$ wasm-pack build --target web
```

**Advantages:**
- Automated dependency management
- TypeScript definitions generated automatically
- Optimizations applied (wasm-opt)
- Single tool for Cargo + WASM + JS integration

**Disadvantages mitigated:**
- Slower builds → acceptable (1st build ~30s, incremental ~1s)
- Adds wasm-pack dependency → acceptable (official tool)

### Alternative: Manual Compilation

```bash
# Would require:
$ rustc --target wasm32-unknown-unknown kernel/src/lib.rs \
    --opt-level=z --lto \
    -C embed-bitcode=yes \
    -o kernel/kernel.wasm
```

**Not chosen because:**
- Lose wasm-bindgen functionality (must write glue by hand)
- Manual optimization flags error-prone
- No TypeScript generation
- Steeper learning curve

### Alternative: Custom Node Build Script

```javascript
// build.js
const { build } = require('esbuild')
const { compile } = require('rust-bindgen')

// Would duplicate wasm-pack functionality
```

**Not chosen because:**
- Reimplementing wasm-pack is wasteful
- Less tested than official tool
- Community has solved this problem

---

## Summary: Design Philosophy

### Core Principles

1. **Correctness over performance**
   - Memory safety prioritized
   - Clear code preferred to optimized code
   - Research OS, not production system

2. **Simplicity over features**
   - Minimum viable OS model
   - Progressive enhancement (v0.2 → v1.0)
   - Educational clarity

3. **Portability over optimization**
   - Browser as hardware abstraction
   - No platform-specific code
   - Runs on Chrome, Firefox, Safari, Edge

4. **Type safety over flexibility**
   - Rust's strong typing catches bugs early
   - JavaScript bridges are narrow and validated
   - Fewer security surprises

### Intentional Simplifications

| Aspect | Simplified For | Production Would Use |
|--------|---|---|
| VFS | In-memory only | Journaling filesystem (ext4, btrfs) |
| Processes | Minimal PCB | Rich resource tracking |
| Scheduling | Cooperative | Preemptive, CPU-aware |
| Security | None | Full capability model |
| Persistence | Optional | Crash-safe, ACID semantics |
| I/O | String-encoded | Binary protocols, shared memory |

These aren't "bugs" but **intentional design decisions** appropriate for a research/educational OS.

---

## Future Decisions (v0.3+)

### Likely Tradeoffs

1. **Persistence**: IndexedDB integration (slower but persistent)
2. **GUI**: Canvas-based desktop (higher code complexity)
3. **Processes**: Support WASM module loading (runtime complexity)
4. **Binary**: Shared memory I/O (manual memory management)
5. **Security**: User/group model (permission checking overhead)

Each will have its own design document explaining tradeoffs.

---

**Next:** Read [ARCHITECTURE.md](ARCHITECTURE.md) for implementation details  
**Questions?** Open an issue with your design question
