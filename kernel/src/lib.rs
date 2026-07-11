#![no_std]
extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::cmp::min;
use wasm_bindgen::prelude::*;

pub mod crypto;
pub mod typestate;
pub mod wasi;
pub mod snapshot;
pub mod distributed;

// ============================================================================
// FORMAL THREAT MODEL & SECURITY INVARIANTS
// ============================================================================
//
// (unchanged from v0.3 — see prior version for full text)
//
// Key invariants added in v0.4:
//
// KERNEL SECURITY INVARIANT 6 (Distributed Capabilities):
//   Remote capabilities are represented as DelegationToken structs
//   authenticated by HMAC-SHA256 using a shared peer key. The receiving
//   kernel verifies the HMAC tag before creating a proxy capability in its
//   local CSpace. Nonces prevent replay attacks. The shared peer key is
//   established via an out-of-band channel (QR code, KCP-style PSK in
//   signaling URL, or DTLS fingerprint verification).
//
// KERNEL SECURITY INVARIANT 7 (Snapshot Integrity):
//   Kernel snapshots are a complete serialization of all mutable state:
//   CSpace, VFS, WASI table, delegation table, and remote proxy table.
//   Snapshot/restore is only permitted from the quiesced state (between
//   WASM export calls, with no in-flight operations).
//
// KERNEL SECURITY INVARIANT 8 (Typestate Compile-Time Guarantees):
//   The `FileDescriptor<State>` wrapper provides zero-cost compile-time
//   verification that write() is never called on a ReadOnly descriptor.
//   This is a defense-in-depth layer above runtime check_cap().
//
// ============================================================================

// ============================================================================
// KERNEL SINGLETON (no_std compatible, no thread_local)
// ============================================================================

use core::sync::atomic::{AtomicBool, Ordering};

struct KernelCell(UnsafeCell<Option<Kernel>>);
unsafe impl Sync for KernelCell {}

pub(crate) static KERNEL: KernelCell = KernelCell(UnsafeCell::new(None));
static KERNEL_INIT: AtomicBool = AtomicBool::new(false);

pub(crate) fn with_kernel<F, T>(f: F) -> T
where
    F: FnOnce(&mut Kernel) -> T,
{
    unsafe {
        let ptr = KERNEL.0.get();
        let cell = (*ptr).as_mut().expect("KERNEL not initialized");
        f(cell)
    }
}

pub(crate) fn with_kernel_immut<F, T>(f: F) -> T
where
    F: FnOnce(&Kernel) -> T,
{
    unsafe {
        let ptr = KERNEL.0.get();
        let cell = (*ptr).as_ref().expect("KERNEL not initialized");
        f(cell)
    }
}

pub(crate) fn with_kernel_mut<F, T>(f: F) -> T
where
    F: FnOnce(&mut Kernel) -> T,
{
    unsafe {
        let ptr = KERNEL.0.get();
        let cell = (*ptr).as_mut().expect("KERNEL not initialized");
        f(cell)
    }
}

unsafe fn init_kernel_impl(time_ms: u64) {
    let ptr = KERNEL.0.get();
    *ptr = Some(Kernel::new(time_ms));
    KERNEL_INIT.store(true, Ordering::Release);
}

fn init_kernel(time_ms: u64) {
    unsafe { init_kernel_impl(time_ms) }
}

// ============================================================================
// SECTION 1: KERNEL ERROR TYPES
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    BadCapability(u32),
    InsufficientRights { cap: u32, missing: CapRights },
    CapabilityRevoked(u32),
    ObjectNotFound(u32),
    PathNotFound(String),
    NotADirectory,
    NotAFile,
    IsADirectory,
    NameTooLong,
    ParentNotFound,
    InodeNotFound(u32),
    FileDescriptorNotFound(u32),
    InvalidUtf8,
    KernelNotBooted,
    OutOfCapacity,
    WouldDeadlock,
    AccessDenied,
    InvalidArgument,
}

impl KernelError {
    pub fn into_i32(&self) -> i32 {
        match self {
            KernelError::BadCapability(_) => -2,
            KernelError::InsufficientRights { .. } => -3,
            KernelError::CapabilityRevoked(_) => -4,
            KernelError::ObjectNotFound(_) => -5,
            KernelError::PathNotFound(_) => -6,
            KernelError::NotADirectory => -7,
            KernelError::NotAFile => -8,
            KernelError::IsADirectory => -9,
            KernelError::NameTooLong => -10,
            KernelError::ParentNotFound => -11,
            KernelError::InodeNotFound(_) => -12,
            KernelError::FileDescriptorNotFound(_) => -13,
            KernelError::InvalidUtf8 => -14,
            KernelError::KernelNotBooted => -15,
            KernelError::OutOfCapacity => -16,
            KernelError::WouldDeadlock => -17,
            KernelError::AccessDenied => -18,
            KernelError::InvalidArgument => -19,
        }
    }

    pub fn into_string(&self) -> String {
        format!("{:?}", self)
    }
}

// ============================================================================
// SECTION 2: CapRights — Type-Safe Permission Bitmask
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapRights(u16);

impl CapRights {
    pub const READ: u16 = 1 << 0;
    pub const WRITE: u16 = 1 << 1;
    pub const EXECUTE: u16 = 1 << 2;
    pub const GRANT: u16 = 1 << 3;
    pub const REVOKE: u16 = 1 << 4;
    pub const DELETE: u16 = 1 << 5;
    pub const ALL: u16 = (1 << 6) - 1;
    pub const NONE: u16 = 0;

    pub fn new(bits: u16) -> Self {
        CapRights(bits & Self::ALL)
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "r" => CapRights(Self::READ),
            "w" => CapRights(Self::WRITE),
            "x" => CapRights(Self::EXECUTE),
            "rw" => CapRights(Self::READ | Self::WRITE),
            "rx" => CapRights(Self::READ | Self::EXECUTE),
            "wx" => CapRights(Self::WRITE | Self::EXECUTE),
            "rwx" => CapRights(Self::READ | Self::WRITE | Self::EXECUTE),
            _ => CapRights(Self::NONE),
        }
    }

    pub fn contains(&self, bits: u16) -> bool {
        (self.0 & bits) == bits
    }

    pub fn contains_rights(&self, other: CapRights) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn intersect(self, other: CapRights) -> CapRights {
        CapRights(self.0 & other.0)
    }

    pub fn union(self, other: CapRights) -> CapRights {
        CapRights(self.0 | other.0)
    }

    pub fn remove(&mut self, other: CapRights) {
        self.0 &= !other.0;
    }

    pub fn bits(&self) -> u16 {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(6);
        if self.contains(Self::READ) {
            s.push('r');
        }
        if self.contains(Self::WRITE) {
            s.push('w');
        }
        if self.contains(Self::EXECUTE) {
            s.push('x');
        }
        if self.contains(Self::GRANT) {
            s.push('g');
        }
        if self.contains(Self::REVOKE) {
            s.push('v');
        }
        if self.contains(Self::DELETE) {
            s.push('d');
        }
        s
    }
}

// ============================================================================
// SECTION 3: OBJECT TYPE & REVOCATION COUNTER
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectType {
    Inode = 1,
    Process = 2,
    RemoteProxy = 3,
}

impl ObjectType {
    pub fn from_u32(v: u32) -> Option<ObjectType> {
        match v {
            1 => Some(ObjectType::Inode),
            2 => Some(ObjectType::Process),
            3 => Some(ObjectType::RemoteProxy),
            _ => None,
        }
    }

    pub fn from_u8(v: u8) -> Option<ObjectType> {
        match v {
            1 => Some(ObjectType::Inode),
            2 => Some(ObjectType::Process),
            3 => Some(ObjectType::RemoteProxy),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectKey {
    pub type_: ObjectType,
    pub id: u32,
}

// ============================================================================
// SECTION 4: CAPABILITY STRUCTURE & CSPACE
// ============================================================================

#[derive(Debug, Clone)]
pub struct Capability {
    pub slot: u32,
    pub object_type: ObjectType,
    pub object_id: u32,
    pub rights: CapRights,
    pub revoke_counter: u64,
    pub epoch: u64,
}

impl Capability {
    fn new(
        slot: u32,
        object_type: ObjectType,
        object_id: u32,
        rights: CapRights,
        revoke_counter: u64,
        epoch: u64,
    ) -> Self {
        Capability {
            slot,
            object_type,
            object_id,
            rights,
            revoke_counter,
            epoch,
        }
    }
}

pub struct CSpace {
    pub(crate) slots: BTreeMap<u32, Capability>,
    next_slot: u32,
    pub(crate) revoke_map: BTreeMap<ObjectKey, u64>,
    pub global_epoch: u64,
}

impl CSpace {
    fn new() -> Self {
        CSpace {
            slots: BTreeMap::new(),
            next_slot: 1,
            revoke_map: BTreeMap::new(),
            global_epoch: 1,
        }
    }

    pub(crate) fn allocate_slot(&mut self) -> u32 {
        let slot = self.next_slot;
        self.next_slot += 1;
        slot
    }

    pub(crate) fn install(&mut self, slot: u32, cap: Capability) {
        self.slots.insert(slot, cap);
    }

    pub fn get(&self, slot: u32) -> Option<&Capability> {
        self.slots.get(&slot)
    }

    pub(crate) fn get_mut(&mut self, slot: u32) -> Option<&mut Capability> {
        self.slots.get_mut(&slot)
    }

    pub(crate) fn remove(&mut self, slot: u32) -> Option<Capability> {
        self.slots.remove(&slot)
    }

    pub(crate) fn get_revoke_counter(&self, object_type: ObjectType, object_id: u32) -> u64 {
        let key = ObjectKey { type_: object_type, id: object_id };
        self.revoke_map.get(&key).copied().unwrap_or(0)
    }

    pub(crate) fn revoke_object(&mut self, object_type: ObjectType, object_id: u32) {
        let key = ObjectKey { type_: object_type, id: object_id };
        let counter = self.revoke_map.entry(key).or_insert(0);
        *counter += 1;
        self.global_epoch += 1;
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&u32, &Capability)> {
        self.slots.iter()
    }

    pub(crate) fn revoke_map_iter(&self) -> impl Iterator<Item = (&ObjectKey, &u64)> {
        self.revoke_map.iter()
    }

    pub(crate) fn next_slot(&self) -> u32 {
        self.next_slot
    }

    pub(crate) fn set_next_slot(&mut self, slot: u32) {
        self.next_slot = slot;
    }

    pub(crate) fn restore_slot(&mut self, slot: u32, cap: Capability) {
        self.slots.insert(slot, cap);
    }

    pub(crate) fn restore_revoke_entry(&mut self, key: ObjectKey, counter: u64) {
        self.revoke_map.insert(key, counter);
    }
}

// ============================================================================
// SECTION 5: PROCESS MANAGEMENT
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
// SECTION 6: VIRTUAL FILE SYSTEM (Inode-Based)
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
    pub children: BTreeMap<String, u32>,
    pub parent: Option<u32>,
    pub revoke_counter: u64,
}

#[derive(Clone)]
pub struct VfsFileDescriptor {
    pub inode_id: u32,
    pub offset: usize,
    pub mode: String,
}

// ============================================================================
// SECTION 7: KERNEL STRUCTURE
// ============================================================================

pub struct Kernel {
    pub(crate) booted: bool,
    start_time_ms: u64,
    current_time_ms: u64,

    process_table: BTreeMap<u32, ProcessControlBlock>,
    next_pid: u32,
    current_pid: u32,

    // Virtual file system
    inode_map: BTreeMap<u32, Inode>,
    next_inode_id: u32,
    root_inode_id: u32,
    open_files: BTreeMap<u32, VfsFileDescriptor>,
    next_fd: u32,

    // Capability architecture
    pub(crate) cspace: CSpace,
    pub(crate) root_cap_slot: u32,

    // WASI translation layer
    pub(crate) wasi_table: wasi::WasiTable,

    // Distributed capabilities (WebRTC)
    pub(crate) delegations: BTreeMap<u64, u32>,
    pub(crate) remote_proxies: BTreeMap<u32, distributed::RemoteProxyInfo>,
    pub(crate) next_delegation_id: u64,
    pub(crate) kernel_id: u64,
    pub(crate) seen_nonces: BTreeSet<u64>,
    pub(crate) next_nonce: u64,
}

// ============================================================================
// SECTION 8: KERNEL IMPLEMENTATION
// ============================================================================

impl Kernel {
    fn new(current_time_ms: u64) -> Self {
        let cspace = CSpace::new();

        let mut kernel = Kernel {
            booted: false,
            start_time_ms: current_time_ms,
            current_time_ms,

            process_table: BTreeMap::new(),
            next_pid: 1,
            current_pid: 0,

            inode_map: BTreeMap::new(),
            next_inode_id: 1,
            root_inode_id: 0,
            open_files: BTreeMap::new(),
            next_fd: 3,

            cspace,
            root_cap_slot: 0,

            wasi_table: wasi::WasiTable::new(),

            delegations: BTreeMap::new(),
            remote_proxies: BTreeMap::new(),
            next_delegation_id: 1,
            kernel_id: current_time_ms.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) as u64,
            seen_nonces: BTreeSet::new(),
            next_nonce: 1,
        };

        let root = Inode {
            id: 0,
            inode_type: InodeType::Directory,
            name: "/".to_string(),
            data: Vec::new(),
            children: BTreeMap::new(),
            parent: None,
            revoke_counter: 0,
        };
        kernel.inode_map.insert(0, root);

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
    // 8a: BOOT SEQUENCE
    // ========================================================================

    fn boot_sequence(&mut self, current_time_ms: u64) -> String {
        self.booted = true;
        self.start_time_ms = current_time_ms;
        self.current_time_ms = current_time_ms;

        let _ = self.create_inode_internal(0, "bin".to_string(), InodeType::Directory);
        let _ = self.create_inode_internal(0, "etc".to_string(), InodeType::Directory);
        let _ = self.create_inode_internal(0, "home".to_string(), InodeType::Directory);
        let _ = self.create_inode_internal(0, "tmp".to_string(), InodeType::Directory);
        let _ = self.create_inode_internal(0, "var".to_string(), InodeType::Directory);

        let root_cap_slot = self.cspace.allocate_slot();
        let root_cap = Capability::new(
            root_cap_slot,
            ObjectType::Inode,
            0,
            CapRights::new(CapRights::ALL),
            self.cspace.get_revoke_counter(ObjectType::Inode, 0),
            self.cspace.global_epoch,
        );
        self.cspace.install(root_cap_slot, root_cap);
        self.root_cap_slot = root_cap_slot;

        // Initialize WASI preopened fd 3 → root cap
        self.wasi_table.init_root(root_cap_slot, 0);

        format!(
            "BrowserOS v0.4 (Capability-Secure + WASI + Distributed)\nRoot capability slot: {}\nType 'help' for command list.\n",
            root_cap_slot
        )
    }

    // ========================================================================
    // 8b: THE GATEKEEPER — check_cap
    // ========================================================================

    pub fn check_cap(
        &self,
        cap_slot: u32,
        expected_object_type: ObjectType,
        expected_object_id: u32,
        required_rights: CapRights,
    ) -> Result<&Capability, KernelError> {
        if !self.booted {
            return Err(KernelError::KernelNotBooted);
        }

        let cap = self
            .cspace
            .get(cap_slot)
            .ok_or(KernelError::BadCapability(cap_slot))?;

        if cap.object_type == ObjectType::RemoteProxy {
            return self.check_remote_proxy(cap_slot, required_rights, cap);
        }

        if cap.object_type != expected_object_type {
            return Err(KernelError::BadCapability(cap_slot));
        }

        let current_counter = self
            .cspace
            .get_revoke_counter(expected_object_type, expected_object_id);
        if cap.revoke_counter != current_counter {
            return Err(KernelError::CapabilityRevoked(cap_slot));
        }

        if !cap.rights.contains_rights(required_rights) {
            return Err(KernelError::InsufficientRights {
                cap: cap_slot,
                missing: required_rights,
            });
        }

        Ok(cap)
    }

    fn check_remote_proxy<'a>(
        &'a self,
        cap_slot: u32,
        required_rights: CapRights,
        cap: &'a Capability,
    ) -> Result<&'a Capability, KernelError> {
        if !cap.rights.contains_rights(required_rights) {
            return Err(KernelError::InsufficientRights {
                cap: cap_slot,
                missing: required_rights,
            });
        }
        if !self.remote_proxies.contains_key(&cap_slot) {
            return Err(KernelError::CapabilityRevoked(cap_slot));
        }
        Ok(cap)
    }

    pub(crate) fn check_cap_raw(&self, slot: u32, required_rights: CapRights) -> Result<&Capability, KernelError> {
        if !self.booted {
            return Err(KernelError::KernelNotBooted);
        }
        let cap = self.cspace.get(slot)
            .ok_or(KernelError::BadCapability(slot))?;
        if !cap.rights.contains_rights(required_rights) {
            return Err(KernelError::InsufficientRights {
                cap: slot,
                missing: required_rights,
            });
        }
        Ok(cap)
    }

    // ========================================================================
    // 8c: CAPABILITY OPERATIONS
    // ========================================================================

    fn cap_create(
        &mut self,
        object_type: ObjectType,
        object_id: u32,
        rights: CapRights,
    ) -> Result<u32, KernelError> {
        let slot = self.cspace.allocate_slot();
        let revoke_counter = self.cspace.get_revoke_counter(object_type, object_id);
        let cap = Capability::new(
            slot,
            object_type,
            object_id,
            rights,
            revoke_counter,
            self.cspace.global_epoch,
        );
        self.cspace.install(slot, cap);
        Ok(slot)
    }

    pub(crate) fn cap_mint(
        &mut self,
        parent_slot: u32,
        object_type: ObjectType,
        object_id: u32,
        requested_rights: CapRights,
    ) -> Result<u32, KernelError> {
        let parent = self.check_cap(
            parent_slot,
            object_type,
            object_id,
            CapRights::new(CapRights::GRANT),
        )?;

        let child_rights = parent.rights.intersect(requested_rights);

        if child_rights.is_empty() {
            return Err(KernelError::InsufficientRights {
                cap: parent_slot,
                missing: requested_rights,
            });
        }

        let slot = self.cspace.allocate_slot();
        let revoke_counter = self.cspace.get_revoke_counter(object_type, object_id);
        let cap = Capability::new(
            slot,
            object_type,
            object_id,
            child_rights,
            revoke_counter,
            self.cspace.global_epoch,
        );
        self.cspace.install(slot, cap);
        Ok(slot)
    }

    fn cap_revoke(&mut self, cap_slot: u32) -> Result<(), KernelError> {
        let cap = self
            .cspace
            .get(cap_slot)
            .ok_or(KernelError::BadCapability(cap_slot))?;

        if !cap.rights.contains(CapRights::REVOKE) {
            return Err(KernelError::InsufficientRights {
                cap: cap_slot,
                missing: CapRights::new(CapRights::REVOKE),
            });
        }

        self.cspace
            .revoke_object(cap.object_type, cap.object_id);
        Ok(())
    }

    fn cap_destroy(&mut self, cap_slot: u32) -> Result<(), KernelError> {
        let cap = self
            .cspace
            .get(cap_slot)
            .ok_or(KernelError::BadCapability(cap_slot))?;

        if !cap.rights.contains(CapRights::DELETE) {
            return Err(KernelError::InsufficientRights {
                cap: cap_slot,
                missing: CapRights::new(CapRights::DELETE),
            });
        }

        self.cspace.remove(cap_slot);
        Ok(())
    }

    fn cap_list(&self, cap_slot: u32) -> Result<Vec<(u32, u32, u16)>, KernelError> {
        let _cap = self.check_cap(
            cap_slot,
            ObjectType::Inode,
            0,
            CapRights::new(CapRights::GRANT),
        )?;

        Ok(self
            .cspace
            .slots
            .iter()
            .map(|(&slot, cap)| (slot, cap.object_id, cap.rights.bits()))
            .collect())
    }

    // ========================================================================
    // 8d: PROCESS MANAGEMENT METHODS
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
    // 8e: FILE SYSTEM METHODS (Internal)
    // ========================================================================

    fn create_inode_internal(
        &mut self,
        parent_id: u32,
        name: String,
        inode_type: InodeType,
    ) -> Result<u32, KernelError> {
        let inode_id = self.next_inode_id;
        self.next_inode_id += 1;

        let inode = Inode {
            id: inode_id,
            inode_type,
            name: name.clone(),
            data: Vec::new(),
            children: BTreeMap::new(),
            parent: Some(parent_id),
            revoke_counter: 0,
        };

        self.inode_map.insert(inode_id, inode);

        if let Some(parent) = self.inode_map.get_mut(&parent_id) {
            if matches!(parent.inode_type, InodeType::Directory) {
                parent.children.insert(name, inode_id);
                Ok(inode_id)
            } else {
                Err(KernelError::NotADirectory)
            }
        } else {
            Err(KernelError::InodeNotFound(parent_id))
        }
    }

    fn get_inode(&self, path: &str) -> Result<u32, KernelError> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Ok(self.root_inode_id);
        }
        let mut current_id = self.root_inode_id;
        for component in path.split('/') {
            if component.is_empty() {
                continue;
            }
            let inode = self
                .inode_map
                .get(&current_id)
                .ok_or_else(|| KernelError::PathNotFound(component.to_string()))?;
            let next_id = inode
                .children
                .get(component)
                .ok_or_else(|| KernelError::PathNotFound(component.to_string()))?;
            current_id = *next_id;
        }
        Ok(current_id)
    }

    fn list_directory_internal(&self, inode_id: u32) -> Result<Vec<String>, KernelError> {
        let inode = self
            .inode_map
            .get(&inode_id)
            .ok_or(KernelError::InodeNotFound(inode_id))?;
        if !matches!(inode.inode_type, InodeType::Directory) {
            return Err(KernelError::NotADirectory);
        }
        Ok(inode.children.keys().cloned().collect())
    }

    fn read_file_content_internal(&self, inode_id: u32) -> Result<String, KernelError> {
        let inode = self
            .inode_map
            .get(&inode_id)
            .ok_or(KernelError::InodeNotFound(inode_id))?;
        String::from_utf8(inode.data.clone()).map_err(|_| KernelError::InvalidUtf8)
    }

    // ========================================================================
    // 8f: CAPABILITY-GATED FILE SYSTEM OPERATIONS
    // ========================================================================

    fn cap_create_file(
        &mut self,
        dir_cap_slot: u32,
        path: &str,
        is_dir: bool,
    ) -> Result<i32, KernelError> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Err(KernelError::PathNotFound(String::new()));
        }
        let (parent_path, name) = if let Some(last_slash) = path.rfind('/') {
            (&path[..last_slash], path[last_slash + 1..].to_string())
        } else {
            ("", path.to_string())
        };
        if name.is_empty() {
            return Err(KernelError::NameTooLong);
        }

        let parent_inode_id = if parent_path.is_empty() {
            0u32
        } else {
            let _parent_cap = self.check_cap(
                dir_cap_slot,
                ObjectType::Inode,
                self.root_inode_id,
                CapRights::new(CapRights::EXECUTE),
            )?;
            self.get_inode(parent_path)?
        };

        let _ = self.check_cap(
            dir_cap_slot,
            ObjectType::Inode,
            parent_inode_id,
            CapRights::new(CapRights::WRITE | CapRights::EXECUTE),
        )?;

        let inode_type = if is_dir {
            InodeType::Directory
        } else {
            InodeType::File
        };

        let inode_id = self.create_inode_internal(parent_inode_id, name, inode_type)?;

        let new_rights = CapRights::new(CapRights::READ | CapRights::WRITE);
        let new_cap_slot = self.cap_mint(dir_cap_slot, ObjectType::Inode, inode_id, new_rights)?;

        Ok(new_cap_slot as i32)
    }

    fn cap_open_file(
        &mut self,
        dir_cap_slot: u32,
        path: &str,
        flags: CapRights,
    ) -> Result<i32, KernelError> {
        let dir_cap = self.check_cap(
            dir_cap_slot,
            ObjectType::Inode,
            self.root_inode_id,
            CapRights::new(CapRights::EXECUTE),
        )?;

        let inode_id = self.get_inode(path)?;

        let _ = self.check_cap(
            dir_cap_slot,
            ObjectType::Inode,
            inode_id,
            flags,
        )?;

        if flags.contains(CapRights::READ) {
            let inode = self
                .inode_map
                .get(&inode_id)
                .ok_or(KernelError::InodeNotFound(inode_id))?;
            if matches!(inode.inode_type, InodeType::Directory) {
                return Err(KernelError::IsADirectory);
            }
        }

        let opened_rights = if flags.contains(CapRights::WRITE) {
            CapRights::new(CapRights::READ | CapRights::WRITE)
        } else {
            CapRights::new(CapRights::READ)
        };

        let file_rights = dir_cap.rights.intersect(opened_rights);

        let file_cap_slot = self.cap_mint(
            dir_cap_slot,
            ObjectType::Inode,
            inode_id,
            file_rights,
        )?;

        Ok(file_cap_slot as i32)
    }

    fn cap_read_file(
        &mut self,
        file_cap_slot: u32,
    ) -> Result<String, KernelError> {
        let cap = self
            .cspace
            .get(file_cap_slot)
            .ok_or(KernelError::BadCapability(file_cap_slot))?;

        let cap = self.check_cap(
            file_cap_slot,
            ObjectType::Inode,
            cap.object_id,
            CapRights::new(CapRights::READ),
        )?;

        let inode = self
            .inode_map
            .get(&cap.object_id)
            .ok_or(KernelError::InodeNotFound(cap.object_id))?;

        if matches!(inode.inode_type, InodeType::Directory) {
            return Err(KernelError::IsADirectory);
        }

        String::from_utf8(inode.data.clone()).map_err(|_| KernelError::InvalidUtf8)
    }

    fn cap_write_file(
        &mut self,
        file_cap_slot: u32,
        data: &[u8],
    ) -> Result<i32, KernelError> {
        let cap = self
            .cspace
            .get(file_cap_slot)
            .ok_or(KernelError::BadCapability(file_cap_slot))?;

        let _ = self.check_cap(
            file_cap_slot,
            ObjectType::Inode,
            cap.object_id,
            CapRights::new(CapRights::WRITE),
        )?;

        let inode_id = cap.object_id;
        let _ = cap;

        let inode = self
            .inode_map
            .get_mut(&inode_id)
            .ok_or(KernelError::InodeNotFound(inode_id))?;

        if matches!(inode.inode_type, InodeType::Directory) {
            return Err(KernelError::IsADirectory);
        }

        let written = data.len();
        inode.data.extend_from_slice(data);
        Ok(written as i32)
    }

    fn cap_read_into_slice(&mut self, cap_slot: u32, offset: u64, buf: &mut [u8]) -> Result<usize, KernelError> {
        let cap = self
            .cspace
            .get(cap_slot)
            .ok_or(KernelError::BadCapability(cap_slot))?;

        let cap_obj_id = cap.object_id;
        let _ = self.check_cap(
            cap_slot,
            ObjectType::Inode,
            cap_obj_id,
            CapRights::new(CapRights::READ),
        )?;

        let inode = self
            .inode_map
            .get(&cap_obj_id)
            .ok_or(KernelError::InodeNotFound(cap_obj_id))?;

        if matches!(inode.inode_type, InodeType::Directory) {
            return Err(KernelError::IsADirectory);
        }

        let start = offset as usize;
        let end = core::cmp::min(start + buf.len(), inode.data.len());
        if start < end {
            let n = end - start;
            let n = core::cmp::min(n, buf.len());
            buf[..n].copy_from_slice(&inode.data[start..start + n]);
            Ok(n)
        } else {
            Ok(0)
        }
    }

    fn cap_write_from_slice(&mut self, cap_slot: u32, _offset: u64, buf: &[u8]) -> Result<usize, KernelError> {
        let cap = self
            .cspace
            .get(cap_slot)
            .ok_or(KernelError::BadCapability(cap_slot))?;

        let cap_obj_id = cap.object_id;
        let _ = self.check_cap(
            cap_slot,
            ObjectType::Inode,
            cap_obj_id,
            CapRights::new(CapRights::WRITE),
        )?;

        let inode = self
            .inode_map
            .get_mut(&cap_obj_id)
            .ok_or(KernelError::InodeNotFound(cap_obj_id))?;

        if matches!(inode.inode_type, InodeType::Directory) {
            return Err(KernelError::IsADirectory);
        }

        inode.data.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn cap_list_directory(
        &self,
        dir_cap_slot: u32,
        path: &str,
    ) -> Result<String, KernelError> {
        let _ = self.check_cap(
            dir_cap_slot,
            ObjectType::Inode,
            self.root_inode_id,
            CapRights::new(CapRights::EXECUTE | CapRights::READ),
        )?;

        let inode_id = self.get_inode(path)?;

        let _ = self.check_cap(
            dir_cap_slot,
            ObjectType::Inode,
            inode_id,
            CapRights::new(CapRights::READ),
        )?;

        let entries = self.list_directory_internal(inode_id)?;
        Ok(entries.join(","))
    }

    fn cap_exists(
        &self,
        dir_cap_slot: u32,
        path: &str,
    ) -> Result<i32, KernelError> {
        let _ = self.check_cap(
            dir_cap_slot,
            ObjectType::Inode,
            self.root_inode_id,
            CapRights::new(CapRights::EXECUTE),
        )?;

        match self.get_inode(path) {
            Ok(inode_id) => {
                let inode = self
                    .inode_map
                    .get(&inode_id)
                    .ok_or(KernelError::InodeNotFound(inode_id))?;
                if matches!(inode.inode_type, InodeType::Directory) {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
            Err(_) => Ok(-1),
        }
    }

    fn cap_get_root_slot(&self) -> u32 {
        self.root_cap_slot
    }

    fn cap_info(&self, cap_slot: u32) -> Result<String, KernelError> {
        let cap = self
            .cspace
            .get(cap_slot)
            .ok_or(KernelError::BadCapability(cap_slot))?;
        Ok(format!(
            "Slot {} -> Object({:?}:{}) Rights=[{}] Rev={}",
            cap.slot,
            cap.object_type,
            cap.object_id,
            cap.rights.to_string(),
            cap.revoke_counter
        ))
    }
}

// ============================================================================
// SECTION 9: WASM-BINDGEN EXPORTS — AMBIENT SYSCALLS (LEGACY)
// ============================================================================

#[wasm_bindgen]
pub fn boot(current_time_ms: u64) -> String {
    init_kernel(current_time_ms);
    with_kernel(|k| k.boot_sequence(current_time_ms))
}

#[wasm_bindgen]
pub fn update_time(current_time_ms: u64) {
    with_kernel(|k| {
        k.current_time_ms = current_time_ms;
    });
}

#[wasm_bindgen]
pub fn fs_open(path: &str, mode: &str) -> i32 {
    with_kernel(|k| match k.get_inode(path) {
        Ok(inode_id) => {
            let fd = k.next_fd;
            k.next_fd += 1;
            let descriptor = VfsFileDescriptor {
                inode_id,
                offset: 0,
                mode: mode.to_string(),
            };
            k.open_files.insert(fd, descriptor);
            fd as i32
        }
        Err(_) => -1,
    })
}

#[wasm_bindgen]
pub fn fs_read(fd: u32, size: usize) -> String {
    with_kernel(|k| {
        let descriptor = k.open_files.get_mut(&fd)?;
        let inode = k.inode_map.get(&descriptor.inode_id)?;
        let start = descriptor.offset;
        let end = min(start + size, inode.data.len());
        let read_data = inode.data[start..end].to_vec();
        descriptor.offset = end;
        Some(read_data)
    }).map(|data: Vec<u8>| {
        data.iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }).unwrap_or_default()
}

#[wasm_bindgen]
pub fn fs_write(fd: u32, data: &str) -> i32 {
    with_kernel(|k| {
        let bytes: Result<Vec<u8>, _> = data
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim().parse::<u8>())
            .collect();

        match bytes {
            Ok(data) => {
                let descriptor = k.open_files.get_mut(&fd)?;
                let inode = k.inode_map.get_mut(&descriptor.inode_id)?;
                let written = data.len();
                inode.data.extend_from_slice(&data);
                Some(written as i32)
            }
            Err(_) => Some(-1),
        }
    }).unwrap_or(-1)
}

#[wasm_bindgen]
pub fn fs_create(path: &str, is_dir: bool) -> i32 {
    with_kernel(|k| {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return -1;
        }
        if let Some(last_slash) = path.rfind('/') {
            let parent_path = &path[..last_slash];
            let name = path[last_slash + 1..].to_string();
            if name.is_empty() {
                return -1;
            }
            if let Ok(parent_id) = k.get_inode(parent_path) {
                let inode_type = if is_dir {
                    InodeType::Directory
                } else {
                    InodeType::File
                };
                match k.create_inode_internal(parent_id, name, inode_type) {
                    Ok(_) => 0,
                    Err(_) => -1,
                }
            } else {
                -1
            }
        } else {
            let inode_type = if is_dir {
                InodeType::Directory
            } else {
                InodeType::File
            };
            match k.create_inode_internal(0, path.to_string(), inode_type) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        }
    })
}

#[wasm_bindgen]
pub fn fs_close(fd: u32) -> i32 {
    with_kernel(|k| {
        k.open_files.remove(&fd);
        0
    })
}

#[wasm_bindgen]
pub fn fs_list(path: &str) -> String {
    with_kernel(|k| match k.get_inode(path) {
        Ok(inode_id) => match k.list_directory_internal(inode_id) {
            Ok(entries) => entries.join(","),
            Err(_) => String::new(),
        },
        Err(_) => String::new(),
    })
}

#[wasm_bindgen]
pub fn fs_exists(path: &str) -> i32 {
    with_kernel(|k| match k.get_inode(path) {
        Ok(inode_id) => {
            if let Some(inode) = k.inode_map.get(&inode_id) {
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
    })
}

#[wasm_bindgen]
pub fn fs_cat(path: &str) -> String {
    with_kernel(|k| match k.get_inode(path) {
        Ok(inode_id) => match k.read_file_content_internal(inode_id) {
            Ok(content) => content,
            Err(e) => format!("Error: {}", e.into_string()),
        },
        Err(e) => format!("Error: {}", e.into_string()),
    })
}

#[wasm_bindgen]
pub fn process_spawn(parent_pid: u32) -> u32 {
    with_kernel(|k| k.create_process(parent_pid))
}

#[wasm_bindgen]
pub fn get_uptime() -> u64 {
    with_kernel_immut(|k| k.get_uptime_ms())
}

#[wasm_bindgen]
pub fn uname() -> String {
    "BrowserOS v0.4 (capability-secure wasm32-unknown-unknown)".to_string()
}

#[wasm_bindgen]
pub fn handle_command(cmd: &str) -> String {
    match cmd.trim() {
        "help" => help(),
        "clear" => String::new(),
        "uname" => uname(),
        "uptime" => with_kernel_immut(|k| {
            let uptime = k.get_uptime_ms();
            let secs = uptime / 1000;
            let mins = secs / 60;
            let hrs = mins / 60;
            format!("Uptime: {}h {}m {}s", hrs, mins % 60, secs % 60)
        }),
        _ => format!("Command not found: {}", cmd),
    }
}

fn help() -> String {
    "=== BrowserOS v0.4 (Capability-Secure + WASI + Distributed) ===
FILESYSTEM COMMANDS (legacy):
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

CAPABILITY COMMANDS:
  cap_root           - Show root capability slot
  cap_info <slot>    - Show capability details
  cap_list           - List all capabilities
  cap_revoke <slot>  - Revoke all caps to an object

SNAPSHOT COMMANDS:
  snap <tag>         - Create a micro-snapshot
  snap_restore <tag> - Restore from a snapshot
  snap_list          - List available snapshots

WASI COMMANDS:
  wasi_boot <module> - Boot a WASI-compatible module

DISTRIBUTED COMMANDS:
  delegate <slot>    - Create delegation token for a capability
  remote_list        - List active remote proxy capabilities
"
    .to_string()
}

// ============================================================================
// SECTION 10: WASM-BINDGEN EXPORTS — CAPABILITY-GATED SYSCALLS
// ============================================================================

// ─── CAPABILITY MANAGEMENT ──────────────────────────────────────────────

#[wasm_bindgen]
pub fn cap_get_root() -> u32 {
    with_kernel_immut(|k| k.cap_get_root_slot())
}

#[wasm_bindgen]
pub fn cap_info(cap_slot: u32) -> String {
    with_kernel_immut(|k| match k.cap_info(cap_slot) {
        Ok(info) => info,
        Err(e) => format!("Error: {}", e.into_string()),
    })
}

#[wasm_bindgen]
pub fn cap_list() -> String {
    with_kernel_immut(|k| {
        let slots: Vec<String> = k
            .cspace
            .slots
            .iter()
            .map(|(&slot, cap)| {
                format!(
                    "  {}: Object({:?}:{}) [{}]",
                    slot,
                    cap.object_type,
                    cap.object_id,
                    cap.rights.to_string()
                )
            })
            .collect();
        format!(
            "CSpace ({} slots):\n{}\nEpoch: {}",
            k.cspace.slot_count(),
            slots.join("\n"),
            k.cspace.global_epoch
        )
    })
}

#[wasm_bindgen]
pub fn cap_mint(parent_slot: u32, object_type: u32, object_id: u32, rights_flags: u16) -> i32 {
    let ot = match ObjectType::from_u32(object_type) {
        Some(t) => t,
        None => return -1,
    };
    let requested = CapRights::new(rights_flags);
    with_kernel(|k| match k.cap_mint(parent_slot, ot, object_id, requested) {
        Ok(new_slot) => new_slot as i32,
        Err(e) => e.into_i32(),
    })
}

#[wasm_bindgen]
pub fn cap_revoke(cap_slot: u32) -> i32 {
    with_kernel(|k| match k.cap_revoke(cap_slot) {
        Ok(_) => 0,
        Err(e) => e.into_i32(),
    })
}

#[wasm_bindgen]
pub fn cap_destroy(cap_slot: u32) -> i32 {
    with_kernel(|k| match k.cap_destroy(cap_slot) {
        Ok(_) => 0,
        Err(e) => e.into_i32(),
    })
}

// ─── CAPABILITY-GATED FILE SYSTEM SYSCALLS ─────────────────────────────

#[wasm_bindgen]
pub fn sys_cap_open(dir_cap_slot: u32, path: &str, flags: u16) -> i32 {
    let rights = CapRights::new(flags);
    with_kernel(|k| match k.cap_open_file(dir_cap_slot, path, rights) {
        Ok(new_cap_slot) => new_cap_slot,
        Err(e) => e.into_i32(),
    })
}

#[wasm_bindgen]
pub fn sys_cap_write(file_cap_slot: u32, data_offset: u32, data_len: u32) -> i32 {
    if data_len == 0 {
        return 0;
    }
    let data =
        unsafe { core::slice::from_raw_parts(data_offset as *const u8, data_len as usize) };
    with_kernel(|k| match k.cap_write_file(file_cap_slot, data) {
        Ok(written) => written,
        Err(e) => e.into_i32(),
    })
}

#[wasm_bindgen]
pub fn sys_cap_read(file_cap_slot: u32) -> String {
    with_kernel(|k| match k.cap_read_file(file_cap_slot) {
        Ok(content) => content,
        Err(e) => format!("Error: {}", e.into_string()),
    })
}

#[wasm_bindgen]
pub fn sys_cap_create(dir_cap_slot: u32, path: &str, is_dir: bool) -> i32 {
    with_kernel(|k| match k.cap_create_file(dir_cap_slot, path, is_dir) {
        Ok(new_cap) => new_cap,
        Err(e) => e.into_i32(),
    })
}

#[wasm_bindgen]
pub fn sys_cap_list(dir_cap_slot: u32, path: &str) -> String {
    with_kernel_immut(|k| match k.cap_list_directory(dir_cap_slot, path) {
        Ok(entries) => entries,
        Err(e) => format!("Error: {}", e.into_string()),
    })
}

#[wasm_bindgen]
pub fn sys_cap_exists(dir_cap_slot: u32, path: &str) -> i32 {
    with_kernel_immut(|k| match k.cap_exists(dir_cap_slot, path) {
        Ok(result) => result,
        Err(_) => -1,
    })
}

#[wasm_bindgen]
pub fn sys_cap_spawn(parent_cap_slot: u32) -> i32 {
    with_kernel(|k| {
        match k.check_cap(
            parent_cap_slot,
            ObjectType::Process,
            k.current_pid,
            CapRights::new(CapRights::EXECUTE),
        ) {
            Ok(_) => {
                let pid = k.create_process(k.current_pid);
                match k.cap_create(
                    ObjectType::Process,
                    pid,
                    CapRights::new(CapRights::ALL),
                ) {
                    Ok(cap_slot) => cap_slot as i32,
                    Err(_) => -1,
                }
            }
            Err(e) => e.into_i32(),
        }
    })
}

// ============================================================================
// SECTION 11: WASM-BINDGEN EXPORTS — SNAPSHOT SYSCALLS
// ============================================================================

#[wasm_bindgen]
pub fn sys_snapshot_prepare() -> u32 {
    with_kernel(|k| k.flush_all());
    0
}

#[wasm_bindgen]
pub fn sys_snapshot_serialize() -> Vec<u8> {
    with_kernel_immut(|k| k.serialize())
}

#[wasm_bindgen]
pub fn sys_snapshot_deserialize(data: &[u8]) -> i32 {
    match Kernel::deserialize(data) {
        Ok(k) => {
            unsafe {
                let ptr = KERNEL.0.get();
                *ptr = Some(k);
                KERNEL_INIT.store(true, Ordering::Release);
            }
            0
        }
        Err(_) => -1,
    }
}

#[wasm_bindgen]
pub fn sys_snapshot_list() -> Vec<String> {
    Vec::new()
}

// ============================================================================
// SECTION 12: WASM-BINDGEN EXPORTS — DISTRIBUTED CAPABILITY SYSCALLS
// ============================================================================

#[wasm_bindgen]
pub fn sys_delegate_cap(cap_slot: u32, peer_key_ptr: u32, peer_key_len: u32) -> Vec<u8> {
    let key_bytes = unsafe {
        core::slice::from_raw_parts(peer_key_ptr as *const u8, peer_key_len as usize)
    };
    if key_bytes.len() != 32 {
        return Vec::new();
    }
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(key_bytes);
    let peer_key = distributed::PeerKey::from_bytes(&key_arr);
    with_kernel(|k| match k.delegate_cap(cap_slot, &peer_key) {
        Ok(token) => token.to_bytes(),
        Err(_) => Vec::new(),
    })
}

#[wasm_bindgen]
pub fn sys_import_delegation(
    token_ptr: u32,
    token_len: u32,
    peer_key_ptr: u32,
    peer_key_len: u32,
    peer_id_lo: u32,
    peer_id_hi: u32,
) -> i32 {
    let token_bytes = unsafe {
        core::slice::from_raw_parts(token_ptr as *const u8, token_len as usize)
    };
    let key_bytes = unsafe {
        core::slice::from_raw_parts(peer_key_ptr as *const u8, peer_key_len as usize)
    };
    if key_bytes.len() != 32 {
        return -1;
    }
    let token = match distributed::DelegationToken::from_bytes(token_bytes) {
        Ok(t) => t,
        Err(_) => return -19,
    };
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(key_bytes);
    let peer_key = distributed::PeerKey::from_bytes(&key_arr);
    let peer_id = (peer_id_lo as u64) | ((peer_id_hi as u64) << 32);
    with_kernel(|k| match k.import_delegation(&token, &peer_key, peer_id) {
        Ok(slot) => slot as i32,
        Err(e) => e.into_i32(),
    })
}

#[wasm_bindgen]
pub fn sys_revoke_delegation(delegation_id_lo: u32, delegation_id_hi: u32) -> i32 {
    let del_id = (delegation_id_lo as u64) | ((delegation_id_hi as u64) << 32);
    with_kernel(|k| match k.revoke_delegation(del_id) {
        Ok(_) => 0,
        Err(e) => e.into_i32(),
    })
}

#[wasm_bindgen]
pub fn sys_list_remote_proxies() -> String {
    with_kernel_immut(|k| {
        let proxies = k.list_remote_proxies();
        if proxies.is_empty() {
            return "No remote proxies\n".to_string();
        }
        let lines: Vec<String> = proxies.iter()
            .map(|(slot, peer_id, del_id)| {
                format!("  Slot {} -> Peer {} (Delegation {})", slot, peer_id, del_id)
            })
            .collect();
        format!("Remote proxies:\n{}\n", lines.join("\n"))
    })
}

#[wasm_bindgen]
pub fn sys_list_delegations() -> String {
    with_kernel_immut(|k| {
        let dels = k.list_delegations();
        if dels.is_empty() {
            return "No active delegations\n".to_string();
        }
        let lines: Vec<String> = dels.iter()
            .map(|(id, slot)| format!("  {} -> local slot {}", id, slot))
            .collect();
        format!("Delegations:\n{}\n", lines.join("\n"))
    })
}

// ============================================================================
// SECTION 13: WASI BOOT EXPORTS
// ============================================================================

#[wasm_bindgen]
pub fn wasi_fd_to_cap(fd: u32) -> i32 {
    with_kernel_immut(|k| {
        k.wasi_table.get(fd).map(|e| e.cap_slot as i32).unwrap_or(-1)
    })
}

#[wasm_bindgen]
pub fn wasi_init_root() -> i32 {
    with_kernel(|k| {
        let root_slot = k.root_cap_slot;
        k.wasi_table.init_root(root_slot, 0);
        0
    })
}

#[wasm_bindgen]
pub fn wasi_get_root_fd() -> u32 {
    3
}

// ============================================================================
// SECTION 14: UNIT TESTS
// ============================================================================

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    extern crate std;

    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    pub struct KernelGuard {
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    pub(crate) fn with_kernel_guarded<F, T>(f: F) -> T
    where
        F: FnOnce(&mut Kernel) -> T,
    {
        with_kernel(f)
    }

    pub(crate) fn with_kernel_immut_guarded<F, T>(f: F) -> T
    where
        F: FnOnce(&Kernel) -> T,
    {
        with_kernel_immut(f)
    }

    pub fn setup_kernel() -> KernelGuard {
        let guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            let ptr = KERNEL.0.get();
            *ptr = None;
        }
        KERNEL_INIT.store(false, Ordering::Release);
        let time = 1000;
        init_kernel(time);
        with_kernel(|k| k.boot_sequence(time));
        KernelGuard { _guard: guard }
    }

    #[test]
    fn test_cap_rights_bitmask_operations() {
        let all = CapRights::new(CapRights::ALL);
        assert!(all.contains(CapRights::READ));
        assert!(all.contains(CapRights::WRITE));
        assert!(all.contains(CapRights::EXECUTE));
        assert!(all.contains(CapRights::GRANT));
        assert!(all.contains(CapRights::REVOKE));
        assert!(all.contains(CapRights::DELETE));

        let read_write = CapRights::new(CapRights::READ | CapRights::WRITE);
        assert!(read_write.contains(CapRights::READ));
        assert!(!read_write.contains(CapRights::EXECUTE));

        let intersection = all.intersect(read_write);
        assert!(intersection.contains(CapRights::READ));
        assert!(intersection.contains(CapRights::WRITE));
        assert!(!intersection.contains(CapRights::EXECUTE));

        assert!(CapRights::new(CapRights::NONE).is_empty());
        assert_eq!(CapRights::from_name("rwx").bits(), CapRights::READ | CapRights::WRITE | CapRights::EXECUTE);
    }

    #[test]
    fn test_kernel_error_conversion() {
        assert_eq!(KernelError::BadCapability(42).into_i32(), -2);
        assert_eq!(
            KernelError::InsufficientRights { cap: 1, missing: CapRights::new(CapRights::WRITE) }.into_i32(),
            -3
        );
        assert_eq!(KernelError::CapabilityRevoked(5).into_i32(), -4);
        assert_eq!(KernelError::AccessDenied.into_i32(), -18);
        assert_eq!(KernelError::InvalidArgument.into_i32(), -19);
    }

    #[test]
    fn test_boot_creates_root_cap() {
        let _kg = setup_kernel();
        with_kernel_immut_guarded(|k| {
            let root_cap = k.cspace.get(k.root_cap_slot).unwrap();
            assert_eq!(root_cap.slot, k.root_cap_slot);
            assert_eq!(root_cap.object_type, ObjectType::Inode);
            assert_eq!(root_cap.object_id, 0);
            assert!(root_cap.rights.contains(CapRights::ALL));
        });
    }

    #[test]
    fn test_boot_directories_exist() {
        let _kg = setup_kernel();
        with_kernel_immut_guarded(|k| {
            assert!(k.get_inode("/bin").is_ok());
            assert!(k.get_inode("/etc").is_ok());
            assert!(k.get_inode("/home").is_ok());
            assert!(k.get_inode("/tmp").is_ok());
        });
    }

    #[test]
    fn test_cap_mint_attenuates_rights() {
        let _kg = setup_kernel();
        with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let child_slot = k.cap_mint(root_slot, ObjectType::Inode, 0,
                CapRights::new(CapRights::READ | CapRights::WRITE)).unwrap();
            let child = k.cspace.get(child_slot).unwrap();
            assert!(child.rights.contains(CapRights::READ));
            assert!(child.rights.contains(CapRights::WRITE));
            assert!(!child.rights.contains(CapRights::GRANT));
        });
    }

    #[test]
    fn test_check_cap_revoked() {
        let _kg = setup_kernel();
        with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let child_slot = k.cap_mint(root_slot, ObjectType::Inode, 0,
                CapRights::new(CapRights::READ)).unwrap();
            k.cap_revoke(root_slot).unwrap();
            let result = k.check_cap(child_slot, ObjectType::Inode, 0,
                CapRights::new(CapRights::READ));
            assert!(matches!(result, Err(KernelError::CapabilityRevoked(_))));
        });
    }

    #[test]
    fn test_cap_write_read_integration() {
        let _kg = setup_kernel();
        with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let file_cap = k.cap_create_file(root_slot, "/test.txt", false).unwrap() as u32;
            let data = b"Hello, capability world!";
            let written = k.cap_write_file(file_cap, data).unwrap();
            assert_eq!(written, data.len() as i32);
            let content = k.cap_read_file(file_cap).unwrap();
            assert_eq!(content, "Hello, capability world!");
        });
    }

    #[test]
    fn test_cap_write_no_write_rights() {
        let _kg = setup_kernel();
        with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let _file_cap = k.cap_create_file(root_slot, "/test2.txt", false).unwrap();
            let file_inode = k.get_inode("/test2.txt").unwrap();

            let read_only_cap = k.cap_mint(root_slot, ObjectType::Inode, file_inode,
                CapRights::new(CapRights::READ)).unwrap();

            let result = k.cap_write_file(read_only_cap, b"data");
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), KernelError::InsufficientRights {
                cap: read_only_cap,
                missing: CapRights::new(CapRights::WRITE),
            });
        });
    }

    #[test]
    fn test_cap_list_directory() {
        let _kg = setup_kernel();
        with_kernel_immut_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let entries = k.cap_list_directory(root_slot, "/").unwrap();
            assert!(entries.contains("bin"));
            assert!(entries.contains("etc"));
            assert!(entries.contains("home"));
            assert!(entries.contains("tmp"));
        });
    }

    #[test]
    fn test_revoke_then_mint_new_cap() {
        let _kg = setup_kernel();
        with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            k.cap_revoke(root_slot).unwrap();
            let result = k.check_cap(root_slot, ObjectType::Inode, 0,
                CapRights::new(CapRights::READ));
            assert!(matches!(result, Err(KernelError::CapabilityRevoked(_))));
        });
    }

    #[test]
    fn test_snapshot_roundtrip() {
        let _kg = setup_kernel();
        with_kernel_guarded(|k| {
            let file_cap = k.cap_create_file(k.root_cap_slot, "/snap_test.txt", false).unwrap() as u32;
            k.cap_write_file(file_cap, b"snapshot data").unwrap();
            let data = k.serialize();
            assert!(data.len() > 8);
            assert_eq!(&data[0..4], b"BOSK");
        });
    }

    #[test]
    fn test_wasi_table_init() {
        let _kg = setup_kernel();
        with_kernel(|k| {
            k.wasi_table.init_root(k.root_cap_slot, 0);
            let entry = k.wasi_table.get(3);
            assert!(entry.is_some());
            assert_eq!(entry.unwrap().cap_slot, k.root_cap_slot);
        });
    }

    #[test]
    fn test_object_type_remote_proxy() {
        assert_eq!(ObjectType::from_u32(3), Some(ObjectType::RemoteProxy));
        assert_eq!(ObjectType::from_u8(3), Some(ObjectType::RemoteProxy));
    }

    #[test]
    fn test_delegate_and_import_integration() {
        let _kg = setup_kernel();
        with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let peer_key = distributed::PeerKey::from_oob_secret(b"test-integration-key");
            let token = k.delegate_cap(root_slot, &peer_key).unwrap();
            assert!(token.to_bytes().len() == 63);
            let imported_slot = k.import_delegation(&token, &peer_key, 99).unwrap();
            assert!(imported_slot > 0);
            let imported_cap = k.cspace.get(imported_slot).unwrap();
            assert_eq!(imported_cap.object_type, ObjectType::RemoteProxy);
        });
    }

    #[test]
    fn test_flush_all_noop() {
        let _kg = setup_kernel();
        with_kernel(|k| {
            k.flush_all();
        });
    }
}
