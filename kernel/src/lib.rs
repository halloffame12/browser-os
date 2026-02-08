use wasm_bindgen::prelude::*;
use std::collections::HashMap;
use std::cell::RefCell;

// ============================================================================
// KERNEL STATE & GLOBALS
// ============================================================================

thread_local! {
    static KERNEL: RefCell<Kernel> = RefCell::new(Kernel::new());
}

// ============================================================================
// PROCESS MANAGEMENT
// ============================================================================

#[derive(Clone, Debug)]
pub enum ProcessState {
    Ready,
    Running,
    Waiting,
    Terminated,
}

#[derive(Clone)]
pub struct ProcessControlBlock {
    pid: u32,
    state: ProcessState,
    parent_pid: Option<u32>,
    exit_code: i32,
}

// ============================================================================
// VIRTUAL FILE SYSTEM
// ============================================================================

#[derive(Clone, Debug)]
pub enum InodeType {
    File,
    Directory,
}

#[derive(Clone)]
pub struct Inode {
    pub id: u32,
    pub inode_type: InodeType,
    pub name: String,
    pub data: Vec<u8>,
    pub children: HashMap<String, u32>,  // For directories: name -> child inode_id
    pub parent: Option<u32>,
}

#[derive(Clone)]
pub struct FileDescriptor {
    pub inode_id: u32,
    pub offset: usize,
    pub mode: String,  // "r" or "w"
}

// ============================================================================
// KERNEL STRUCTURE
// ============================================================================

pub struct Kernel {
    booted: bool,
    start_time_ms: u64,
    current_time_ms: u64,
    
    // Process management
    process_table: HashMap<u32, ProcessControlBlock>,
    next_pid: u32,
    current_pid: u32,
    
    // Virtual file system
    inode_map: HashMap<u32, Inode>,
    next_inode_id: u32,
    root_inode_id: u32,
    open_files: HashMap<u32, FileDescriptor>,
    next_fd: u32,
}

impl Kernel {
    fn new() -> Self {
        let mut kernel = Kernel {
            booted: false,
            start_time_ms: 0,
            current_time_ms: 0,
            process_table: HashMap::new(),
            next_pid: 1,
            current_pid: 0,
            inode_map: HashMap::new(),
            next_inode_id: 1,
            root_inode_id: 0,
            open_files: HashMap::new(),
            next_fd: 3,  // 0, 1, 2 are stdin, stdout, stderr
        };
        
        // Initialize filesystem with root directory
        let root = Inode {
            id: 0,
            inode_type: InodeType::Directory,
            name: "/".to_string(),
            data: vec![],
            children: HashMap::new(),
            parent: None,
        };
        kernel.inode_map.insert(0, root);
        
        // Create initial process (init)
        let init_pcb = ProcessControlBlock {
            pid: 0,
            state: ProcessState::Running,
            parent_pid: None,
            exit_code: 0,
        };
        kernel.process_table.insert(0, init_pcb);
        kernel.next_pid = 1;
        kernel.current_pid = 0;
        
        kernel
    }

    // ========================================================================
    // PROCESS MANAGEMENT METHODS
    // ========================================================================

    fn create_process(&mut self, parent_pid: u32) -> u32 {
        let pid = self.next_pid;
        self.next_pid += 1;
        
        let pcb = ProcessControlBlock {
            pid,
            state: ProcessState::Ready,
            parent_pid: Some(parent_pid),
            exit_code: 0,
        };
        
        self.process_table.insert(pid, pcb);
        pid
    }

    fn get_uptime_ms(&self) -> u64 {
        if self.booted && self.current_time_ms >= self.start_time_ms {
            self.current_time_ms - self.start_time_ms
        } else {
            0
        }
    }

    // ========================================================================
    // FILE SYSTEM METHODS
    // ========================================================================

    fn create_inode(&mut self, parent_id: u32, name: String, inode_type: InodeType) -> Result<u32, String> {
        let inode_id = self.next_inode_id;
        self.next_inode_id += 1;
        
        let inode = Inode {
            id: inode_id,
            inode_type,
            name: name.clone(),
            data: vec![],
            children: HashMap::new(),
            parent: Some(parent_id),
        };
        
        self.inode_map.insert(inode_id, inode);
        
        // Add to parent directory
        if let Some(parent) = self.inode_map.get_mut(&parent_id) {
            if matches!(parent.inode_type, InodeType::Directory) {
                parent.children.insert(name, inode_id);
                Ok(inode_id)
            } else {
                Err("Parent is not a directory".to_string())
            }
        } else {
            Err("Parent inode not found".to_string())
        }
    }

    fn get_inode(&self, path: &str) -> Result<u32, String> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Ok(self.root_inode_id);
        }
        
        let mut current_id = self.root_inode_id;
        
        for component in path.split('/') {
            if component.is_empty() {
                continue;
            }
            
            if let Some(inode) = self.inode_map.get(&current_id) {
                if let Some(&next_id) = inode.children.get(component) {
                    current_id = next_id;
                } else {
                    return Err(format!("Path component not found: {}", component));
                }
            } else {
                return Err("Inode not found during traversal".to_string());
            }
        }
        
        Ok(current_id)
    }

    fn open_file(&mut self, inode_id: u32, mode: &str) -> Result<u32, String> {
        if !self.inode_map.contains_key(&inode_id) {
            return Err("Inode not found".to_string());
        }
        
        let fd = self.next_fd;
        self.next_fd += 1;
        
        let descriptor = FileDescriptor {
            inode_id,
            offset: 0,
            mode: mode.to_string(),
        };
        
        self.open_files.insert(fd, descriptor);
        Ok(fd)
    }

    fn read_file(&mut self, fd: u32, buf_size: usize) -> Result<Vec<u8>, String> {
        if let Some(descriptor) = self.open_files.get_mut(&fd) {
            if let Some(inode) = self.inode_map.get(&descriptor.inode_id) {
                let start = descriptor.offset;
                let end = std::cmp::min(start + buf_size, inode.data.len());
                let read_data = inode.data[start..end].to_vec();
                descriptor.offset = end;
                Ok(read_data)
            } else {
                Err("Inode not found".to_string())
            }
        } else {
            Err("File descriptor not found".to_string())
        }
    }

    fn write_file(&mut self, fd: u32, data: &[u8]) -> Result<usize, String> {
        if let Some(descriptor) = self.open_files.get_mut(&fd) {
            if let Some(inode) = self.inode_map.get_mut(&descriptor.inode_id) {
                let written = data.len();
                inode.data.extend_from_slice(data);
                Ok(written)
            } else {
                Err("Inode not found".to_string())
            }
        } else {
            Err("File descriptor not found".to_string())
        }
    }

    fn close_file(&mut self, fd: u32) -> Result<(), String> {
        if self.open_files.remove(&fd).is_some() {
            Ok(())
        } else {
            Err("File descriptor not found".to_string())
        }
    }

    fn list_directory(&self, inode_id: u32) -> Result<Vec<String>, String> {
        if let Some(inode) = self.inode_map.get(&inode_id) {
            if matches!(inode.inode_type, InodeType::Directory) {
                let entries: Vec<String> = inode.children.keys().cloned().collect();
                Ok(entries)
            } else {
                Err("Not a directory".to_string())
            }
        } else {
            Err("Inode not found".to_string())
        }
    }

    fn read_file_content(&self, inode_id: u32) -> Result<String, String> {
        if let Some(inode) = self.inode_map.get(&inode_id) {
            match String::from_utf8(inode.data.clone()) {
                Ok(s) => Ok(s),
                Err(_) => Err("File content is not valid UTF-8".to_string()),
            }
        } else {
            Err("Inode not found".to_string())
        }
    }
}

// ============================================================================
// WASM-BINDGEN EXPORTS & SYSCALLS
// ============================================================================

#[wasm_bindgen]
pub fn boot(current_time_ms: u64) -> String {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        kernel.booted = true;
        kernel.start_time_ms = current_time_ms;
        kernel.current_time_ms = current_time_ms;
        
        // Create initial filesystem structure
        let _ = kernel.create_inode(0, "bin".to_string(), InodeType::Directory);
        let _ = kernel.create_inode(0, "etc".to_string(), InodeType::Directory);
        let _ = kernel.create_inode(0, "home".to_string(), InodeType::Directory);
        let _ = kernel.create_inode(0, "tmp".to_string(), InodeType::Directory);
        
        "BrowserOS v0.2 (WASM-based virtual OS)\nType 'help' for command list.\n".to_string()
    })
}

#[wasm_bindgen]
pub fn update_time(current_time_ms: u64) {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        kernel.current_time_ms = current_time_ms;
    });
}

// Syscall: fs_open(path, mode) -> fd
#[wasm_bindgen]
pub fn fs_open(path: &str, mode: &str) -> i32 {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        match kernel.get_inode(path) {
            Ok(inode_id) => {
                match kernel.open_file(inode_id, mode) {
                    Ok(fd) => fd as i32,
                    Err(_) => -1,
                }
            }
            Err(_) => -1,
        }
    })
}

// Syscall: fs_read(fd, size) -> data as comma-separated bytes
#[wasm_bindgen]
pub fn fs_read(fd: u32, size: usize) -> String {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        match kernel.read_file(fd, size) {
            Ok(data) => {
                data.iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            }
            Err(_) => "".to_string(),
        }
    })
}

// Syscall: fs_write(fd, data as comma-separated bytes) -> bytes_written
#[wasm_bindgen]
pub fn fs_write(fd: u32, data: &str) -> i32 {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        let bytes: Result<Vec<u8>, _> = data
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().parse::<u8>())
            .collect();
        
        match bytes {
            Ok(data) => match kernel.write_file(fd, &data) {
                Ok(written) => written as i32,
                Err(_) => -1,
            },
            Err(_) => -1,
        }
    })
}

// Syscall: fs_create(path, type) -> 0 on success, -1 on error
#[wasm_bindgen]
pub fn fs_create(path: &str, is_dir: bool) -> i32 {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        
        // Validate path
        let path = path.trim_matches('/');
        if path.is_empty() {
            return -1;  // Cannot create root or with empty name
        }
        
        if let Some(last_slash) = path.rfind('/') {
            let parent_path = &path[..last_slash];
            let name = path[last_slash + 1..].to_string();
            
            // Validate name is not empty
            if name.is_empty() {
                return -1;
            }
            
            if let Ok(parent_id) = kernel.get_inode(parent_path) {
                let inode_type = if is_dir { InodeType::Directory } else { InodeType::File };
                match kernel.create_inode(parent_id, name, inode_type) {
                    Ok(_) => 0,
                    Err(_) => -1,
                }
            } else {
                -1
            }
        } else {
            // File in root
            let inode_type = if is_dir { InodeType::Directory } else { InodeType::File };
            match kernel.create_inode(0, path.to_string(), inode_type) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        }
    })
}

// Syscall: fs_close(fd) -> 0 on success, -1 on error
#[wasm_bindgen]
pub fn fs_close(fd: u32) -> i32 {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        match kernel.close_file(fd) {
            Ok(_) => 0,
            Err(_) => -1,
        }
    })
}

// Syscall: fs_list(path) -> comma-separated filenames
#[wasm_bindgen]
pub fn fs_list(path: &str) -> String {
    KERNEL.with(|k| {
        let kernel = k.borrow();
        match kernel.get_inode(path) {
            Ok(inode_id) => {
                match kernel.list_directory(inode_id) {
                    Ok(entries) => entries.join(","),
                    Err(_) => String::new(),
                }
            }
            Err(_) => String::new(),
        }
    })
}

// Syscall: fs_exists(path) -> 1 if dir exists, 0 if file exists, -1 if not found
#[wasm_bindgen]
pub fn fs_exists(path: &str) -> i32 {
    KERNEL.with(|k| {
        let kernel = k.borrow();
        match kernel.get_inode(path) {
            Ok(inode_id) => {
                if let Some(inode) = kernel.inode_map.get(&inode_id) {
                    if matches!(inode.inode_type, InodeType::Directory) {
                        1
                    } else {
                        0
                    }
                } else {
                    -1
                }
            }
            Err(_) => -1,
        }
    })
}

// Syscall: fs_cat(path) -> file content
#[wasm_bindgen]
pub fn fs_cat(path: &str) -> String {
    KERNEL.with(|k| {
        let kernel = k.borrow();
        match kernel.get_inode(path) {
            Ok(inode_id) => {
                match kernel.read_file_content(inode_id) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            }
            Err(e) => format!("Error: {}", e),
        }
    })
}

// Syscall: process_spawn(parent_pid) -> new_pid
#[wasm_bindgen]
pub fn process_spawn(parent_pid: u32) -> u32 {
    KERNEL.with(|k| {
        let mut kernel = k.borrow_mut();
        kernel.create_process(parent_pid)
    })
}

// Syscall: get_uptime() -> uptime in milliseconds
#[wasm_bindgen]
pub fn get_uptime() -> u64 {
    KERNEL.with(|k| {
        let kernel = k.borrow();
        kernel.get_uptime_ms()
    })
}

// Syscall: uname() -> system info
#[wasm_bindgen]
pub fn uname() -> String {
    "BrowserOS v0.2 (wasm32-unknown-unknown)".to_string()
}

#[wasm_bindgen]
pub fn handle_command(cmd: &str) -> String {
    match cmd.trim() {
        "help" => help(),
        "clear" => "".to_string(),
        "uname" => uname(),
        "uptime" => {
            KERNEL.with(|k| {
                let kernel = k.borrow();
                let uptime = kernel.get_uptime_ms();
                let secs = uptime / 1000;
                let mins = secs / 60;
                let hrs = mins / 60;
                format!("Uptime: {}h {}m {}s", hrs, mins % 60, secs % 60)
            })
        }
        _ => format!("Command not found: {}", cmd),
    }
}

fn help() -> String {
"=== BrowserOS Shell ===
FILESYSTEM COMMANDS:
  ls [path]          - List directory contents
  cat [path]         - Print file contents
  touch [path]       - Create an empty file
  mkdir [path]       - Create a directory
  echo TEXT > FILE   - Write text to file

SYSTEM COMMANDS:
  help               - Show this help
  clear              - Clear terminal
  uname              - Show system info
  uptime             - Show uptime

PROCESS COMMANDS:
  ps                 - List processes
  
TIPS:
  - Paths start with / (e.g., /home/user/file.txt)
  - Use '>' for redirection: echo hello > /tmp/test.txt
".to_string()
}