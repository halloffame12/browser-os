# BrowserOS: Extension Guide

Practical examples showing how to extend BrowserOS with new features.

---

## Part 1: Adding New Shell Commands

### Example 1: Simple Output Command (`date`)

**Goal:** Add a `date` command that shows the current date and time.

#### Step 1: Add JavaScript handler in main.js

```javascript
// In Shell.execute() method, add:
case "date":
    output = new Date().toString();
    break;
```

#### Step 2: Test

```
$ date
Wed Feb 08 2026 14:32:15 GMT+0000 (Coordinated Universal Time)
```

**This is the simplest possible extension.**

---

### Example 2: Command with Arguments (`mkdir`)

**Goal:** Create directory with path validation.

#### Current Implementation (Already Done)

```javascript
case "mkdir":
    output = await this.cmdMKDIR(args);
    break;

async cmdMKDIR(args) {
    if (args.length === 0) {
        return "mkdir: missing operand";
    }
    const path = args[0];
    const result = kernel.fs_create(path, true);
    if (result === 0) {
        return `Created directory: ${path}`;
    } else {
        return `mkdir: cannot create '${path}'`;
    }
}
```

**Pattern:**
1. Check argument count
2. Call kernel syscall
3. Check result (0 = success, -1 = error)
4. Return user-friendly message

#### Extension: Add `-p` (parents) flag

```javascript
async cmdMKDIR(args) {
    let createParents = false;
    let path = args[0];
    
    // Parse flags
    if (path === "-p" && args.length > 1) {
        createParents = true;
        path = args[1];
    }
    
    if (!path) {
        return "mkdir: missing operand";
    }
    
    if (createParents) {
        // Create all parent directories
        const dirs = path.split("/").filter(d => d);
        let currentPath = "";
        for (const dir of dirs) {
            currentPath += "/" + dir;
            const result = kernel.fs_create(currentPath, true);
            if (result !== 0 && !this.dirExists(currentPath)) {
                return `mkdir: cannot create '${currentPath}'`;
            }
        }
        return `Created path: ${path}`;
    } else {
        const result = kernel.fs_create(path, true);
        return result === 0 ? `Created: ${path}` : `mkdir: cannot create '${path}'`;
    }
}

dirExists(path) {
    const list = kernel.fs_list(path);
    return list !== null && list !== "";
}
```

---

### Example 3: Pipe Operations (`echo ... | grep ...`)

**Goal:** Support output piping between commands.

#### Step 1: Modify shell parser to detect pipes

```javascript
async execute(cmdLine) {
    // Handle pipes: "cmd1 | cmd2 | cmd3"
    if (cmdLine.includes("|")) {
        return await this.executePipeline(cmdLine);
    }
    
    // ... existing redirect handling
}
```

#### Step 2: Implement pipeline executor

```javascript
async executePipeline(cmdLine) {
    const commands = cmdLine.split("|").map(c => c.trim());
    let output = "";
    
    for (const cmd of commands) {
        if (output) {
            // Pass previous output as stdin
            const prevOutput = output;
            output = await this.executeWithInput(cmd, prevOutput);
        } else {
            // First command has no input
            output = await this.execute(cmd);
        }
    }
    
    return output;
}

async executeWithInput(cmdLine, input) {
    const parts = cmdLine.split(/\s+/);
    const cmd = parts[0];
    
    switch (cmd) {
        case "grep":
            return this.grepFilter(input, parts[1]);
        case "wc":
            return this.wordCount(input);
        default:
            return input;  // Passthrough
    }
}

grepFilter(text, pattern) {
    return text.split("\n")
        .filter(line => line.includes(pattern))
        .join("\n");
}
```

#### Test:

```
$ echo -e "apple\nbanana\napricot" | grep "ap"
apple
apricot
```

---

## Part 2: Adding New Syscalls

### Example 1: `sys_whoami()` - Get Current User

**Goal:** Add a syscall that returns the current user (hardcoded to demonstrate).

#### Step 1: Add Rust implementation

```rust
// kernel/src/lib.rs

#[wasm_bindgen]
pub fn sys_whoami() -> String {
    "nobody".to_string()  // Future: track actual user in kernel state
}
```

#### Step 2: Expose in JavaScript

```javascript
// web/main.js - no extra work needed!
// JavaScript automatically gets this function

// Usage in shell:
case "whoami":
    output = kernel.sys_whoami();
    break;
```

#### Step 3: Test

```
$ whoami
nobody
```

---

### Example 2: `sys_memory_usage()` - Get System Memory

**Goal:** Return statistics about memory usage.

#### Step 1: Add kernel-side tracking

```rust
// Enhance Kernel struct:
pub struct Kernel {
    // ... existing fields
    total_memory: usize,  // WASM memory limit
    allocated_memory: usize,  // Currently used
}

impl Kernel {
    fn new() -> Self {
        let mut kernel = Kernel {
            // ... existing init
            total_memory: 10_000_000,  // 10 MB initial estimate
            allocated_memory: 0,
        };
        
        // Calculate actual usage
        kernel.update_memory_usage();
        kernel
    }
    
    fn update_memory_usage(&mut self) {
        // Sum all inode data sizes
        let mut used = 0;
        for inode in self.inode_map.values() {
            used += inode.data.len();
        }
        self.allocated_memory = used;
    }
}

#[wasm_bindgen]
pub fn sys_memory_info() -> String {
    KERNEL.with(|k| {
        let kernel = k.borrow_mut();
        let used = kernel.allocated_memory;
        let total = kernel.total_memory;
        let percent = (used * 100) / total;
        
        format!("Memory: {} KB / {} KB ({}%)",
            used / 1024,
            total / 1024,
            percent)
    })
}
```

#### Step 2: Use in shell

```javascript
case "free":
    output = kernel.sys_memory_info();
    break;
```

#### Test:

```
$ free
Memory: 1024 KB / 10240 KB (10%)
```

---

### Example 3: Async Syscall Example (`sys_fetch()`)

**Goal:** Add a syscall that fetches HTTP data (demonstrates JavaScript ↔ Rust async).

#### Step 1: Create JavaScript helper

```javascript
// web/main.js - add to device drivers

class Network {
    async fetch(url) {
        try {
            const response = await fetch(url);
            return await response.text();
        } catch (e) {
            return `Error: ${e.message}`;
        }
    }
}

// In main():
const network = new Network();
```

#### Step 2: Add Rust syscall that returns a promise

```rust
// kernel/src/lib.rs

#[wasm_bindgen(raw_module = "./network.js")]
extern "C" {
    #[wasm_bindgen(js_name = fetchURL)]
    async fn fetch_url(url: &str) -> String;
}

#[wasm_bindgen]
pub async fn sys_fetch(url: &str) -> String {
    fetch_url(url).await
}
```

#### Step 3: Create JavaScript bridge

```javascript
// web/network.js

export async function fetchURL(url) {
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
    }
    return await response.text();
}
```

#### Step 4: Use in shell

```javascript
case "fetch":
    output = await kernel.sys_fetch(args[0]);
    break;

// Test:
// $ fetch https://example.com/api/data.json
```

---

## Part 3: File System Enhancements

### Example 1: File Permissions

**Goal:** Add read/write/execute permissions to files.

#### Step 1: Enhance Inode structure

```rust
#[derive(Clone)]
pub struct Inode {
    // ... existing fields
    permissions: u16,  // Unix-style: 0755, 0644, etc
    owner: String,  // Username
}

// Helper functions:
impl Inode {
    fn is_readable(&self, user: &str) -> bool {
        // Check if user can read this inode
        // TODO: Implement permission checking
        true  // For now, all readable
    }
    
    fn is_writable(&self, user: &str) -> bool {
        true  // For now, all writable
    }
}
```

#### Step 2: Add syscalls for permission management

```rust
#[wasm_bindgen]
pub fn sys_chmod(path: &str, mode: u16) -> i32 {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        match kernel.get_inode(path) {
            Ok(inode_id) => {
                if let Some(inode) = kernel.inode_map.get_mut(&inode_id) {
                    inode.permissions = mode;
                    0  // Success
                } else {
                    -1
                }
            }
            Err(_) => -1,
        }
    })
}

#[wasm_bindgen]
pub fn sys_chown(path: &str, owner: &str) -> i32 {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        match kernel.get_inode(path) {
            Ok(inode_id) => {
                if let Some(inode) = kernel.inode_map.get_mut(&inode_id) {
                    inode.owner = owner.to_string();
                    0
                } else {
                    -1
                }
            }
            Err(_) => -1,
        }
    })
}
```

#### Step 3: Create shell commands

```javascript
case "chmod":
    if (args.length < 2) return "chmod: missing arguments";
    const mode = parseInt(args[0], 8);  // Parse octal
    const path = args[1];
    kernel.sys_chmod(path, mode);
    output = `Changed ${path} to ${args[0]}`;
    break;

case "chown":
    if (args.length < 2) return "chown: missing arguments";
    const owner = args[0];
    const path = args[1];
    kernel.sys_chown(path, owner);
    output = `Changed owner of ${path} to ${owner}`;
    break;
```

---

### Example 2: File Search (`.find()`)

**Goal:** Add `find` command to recursively search filesystem.

#### Step 1: Add kernel helper

```rust
#[wasm_bindgen]
pub fn sys_find(path: &str, pattern: &str) -> String {
    KERNEL.with(|k| {
        let kernel = k.borrow();
        let mut results = Vec::new();
        
        fn search(
            kernel: &Kernel,
            inode_id: u32,
            current_path: &str,
            pattern: &str,
            results: &mut Vec<String>
        ) {
            if let Some(inode) = kernel.inode_map.get(&inode_id) {
                // Check if current directory matches pattern
                if current_path.contains(pattern) {
                    results.push(current_path.to_string());
                }
                
                // Recurse into children
                for (name, child_id) in &inode.children {
                    let child_path = if current_path.ends_with('/') {
                        format!("{}{}", current_path, name)
                    } else {
                        format!("{}/{}", current_path, name)
                    };
                    search(kernel, *child_id, &child_path, pattern, results);
                }
            }
        }
        
        if let Ok(root_id) = kernel.get_inode(path) {
            search(&kernel, root_id, path, pattern, &mut results);
        }
        
        results.join("\n")
    })
}
```

#### Step 2: Shell command

```javascript
case "find":
    if (args.length < 2) return "find: usage: find PATH PATTERN";
    const path = args[0];
    const pattern = args[1];
    output = kernel.sys_find(path, pattern);
    break;

// Test:
// $ find / .txt
// /home/user/file.txt
// /tmp/note.txt
```

---

## Part 4: Process Management Extensions

### Example 1: Process Listing with Details

**Goal:** Enhance `ps` command to show more information.

#### Step 1: Add kernel syscall

```rust
#[wasm_bindgen]
pub fn sys_ps() -> String {
    KERNEL.with(|k| {
        let kernel = k.borrow();
        let mut output = String::from("PID  STATE      PPID  COMMAND\n");
        output.push_str("---  -----      ----  -------\n");
        
        for (pid, pcb) in &kernel.process_table {
            let state_str = format!("{:?}", pcb.state);
            let ppid = pcb.parent_pid.map(|p| p.to_string())
                .unwrap_or_else(|| "-".to_string());
            let cmd = format!("[process:{}]", pid);
            
            output.push_str(&format!("{:<3}  {:<10} {:<4}  {}\n",
                pid, state_str, ppid, cmd));
        }
        
        output
    })
}
```

#### Step 2: Use in shell

```javascript
case "ps":
    output = kernel.sys_ps();
    break;
```

---

### Example 2: Process Spawning

**Goal:** Implement `exec` command to spawn new processes (when WASM modules exist).

#### Step 1: Enhance kernel syscall

```rust
#[wasm_bindgen]
pub fn sys_exec(program_path: &str) -> u32 {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        
        // Check if program exists
        if let Ok(_inode_id) = kernel.get_inode(program_path) {
            // Create new process
            let pid = kernel.create_process(0);  // Parent is init
            // In future: load WASM binary and execute
            pid
        } else {
            u32::MAX  // Error code
        }
    })
}
```

#### Step 2: Shell command

```javascript
case "exec":
    if (args.length === 0) return "exec: missing program";
    const pid = kernel.sys_exec(args[0]);
    output = (pid !== 0xFFFFFFFF)
        ? `Spawned process ${pid}`
        : `exec: ${args[0]}: not found`;
    break;
```

---

## Part 5: Persistence & Storage

### Example 1: Save/Load Filesystem Snapshot

**Goal:** Implement `save` and `load` commands for persistence.

#### Step 1: Add kernel serialization

```rust
use serde_json::json;

#[wasm_bindgen]
pub fn sys_save_state(filename: &str) -> i32 {
    KERNEL.with(|k| {
        let kernel = k.borrow();
        
        // Serialize inodes
        let mut inode_list = Vec::new();
        for (id, inode) in &kernel.inode_map {
            inode_list.push(json!({
                "id": id,
                "name": inode.name,
                "type": format!("{:?}", inode.inode_type),
                "data": inode.data.clone(),
                "children": inode.children.clone(),
                "parent": inode.parent,
            }));
        }
        
        let snapshot = json!({
            "version": "0.2",
            "inodes": inode_list,
            "next_inode_id": kernel.next_inode_id,
        });
        
        let json_str = snapshot.to_string();
        
        // Save to file in VFS
        match kernel.get_inode("/tmp") {
            Ok(tmp_id) => {
                // Would need to write JSON to file
                0
            }
            Err(_) => -1,
        }
    })
}
```

#### Step 2: JavaScript integration with IndexedDB

```javascript
case "save":
    output = "Saving filesystem state...";
    const state = kernel.sys_save_state("/tmp/state.json");
    if (state === 0) {
        await storage.save("vfs_snapshot_" + Date.now(), 
            kernel.serialize_vfs());
        output += "\n✓ Saved to IndexedDB";
    } else {
        output += "\n✗ Save failed";
    }
    break;

case "load":
    output = "Loading filesystem state...";
    const snapshot = await storage.load("vfs_snapshot");
    if (snapshot) {
        kernel.restore_vfs(snapshot);
        output += "\n✓ Loaded from IndexedDB";
    } else {
        output += "\n✗ No saved state found";
    }
    break;
```

---

## Part 6: Testing Extensions

### Unit Tests (Adding to Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_inode_creation() {
        let mut kernel = Kernel::new();
        let inode_id = kernel.create_inode(
            0,
            "test.txt".to_string(),
            InodeType::File
        ).unwrap();
        
        assert!(inode_id > 0);
        assert!(kernel.inode_map.contains_key(&inode_id));
    }
    
    #[test]
    fn test_path_traversal() {
        let mut kernel = Kernel::new();
        
        // Setup: /home/user/file.txt
        let home_id = kernel.create_inode(0, "home".to_string(), 
            InodeType::Directory).unwrap();
        let user_id = kernel.create_inode(home_id, "user".to_string(),
            InodeType::Directory).unwrap();
        let file_id = kernel.create_inode(user_id, "file.txt".to_string(),
            InodeType::File).unwrap();
        
        // Test lookup
        let found = kernel.get_inode("/home/user/file.txt").unwrap();
        assert_eq!(found, file_id);
    }
}
```

### Integration Tests (JavaScript)

```javascript
// test/integration.test.js (using Jest)

describe("BrowserOS Shell", () => {
    
    test("creates and reads file", async () => {
        await shell.execute("touch /tmp/test.txt");
        await shell.execute("echo hello > /tmp/test.txt");
        
        // Manually write for now (echo > not fully implemented)
        const fd = kernel.fs_open("/tmp/test.txt", "w");
        kernel.fs_write(fd, "104,101,108,108,111");  // "hello" in bytes
        kernel.fs_close(fd);
        
        const content = kernel.fs_cat("/tmp/test.txt");
        expect(content).toContain("hello");
    });
    
    test("lists directory", async () => {
        const list = kernel.fs_list("/");
        expect(list).toContain("bin");
        expect(list).toContain("tmp");
    });
});
```

**Run tests:**
```bash
jest test/integration.test.js
```

---

## Summary of Extension Patterns

| Task | Pattern | Complexity |
|------|---------|-----------|
| Add simple command | JS case statement | ⭐ |
| Add syscall | Rust #[wasm_bindgen] + JS wrapper | ⭐⭐ |
| Add filesystem feature | Inode enhancement | ⭐⭐ |
| Add persistence | IndexedDB + serialization | ⭐⭐⭐ |
| Add WASM execution | Module loading + sandboxing | ⭐⭐⭐⭐ |

---

## Recommended Starter Projects

1. **Add `wc` command** - Word/line count  
2. **Add `head` / `tail` commands** - File truncation  
3. **Add `tree` command** - Visualize directory structure  
4. **Add soft links** - Symbolic links to files  
5. **Add persistence** - Save/load filesystem with IndexedDB  

---

**Questions about extensions?** See [ARCHITECTURE.md](ARCHITECTURE.md) for kernel internals.
