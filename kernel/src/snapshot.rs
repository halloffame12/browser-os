use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::convert::TryInto;
use crate::{CapRights, ObjectType, KernelError, Capability, ObjectKey, Inode, InodeType, Kernel};

const SNAPSHOT_MAGIC: &[u8; 4] = b"BOSK";
const SNAPSHOT_VERSION: u32 = 0x0004_0000;

pub struct SnapshotWriter {
    buf: Vec<u8>,
}

impl SnapshotWriter {
    pub fn new() -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(SNAPSHOT_MAGIC);
        buf.extend_from_slice(&SNAPSHOT_VERSION.to_le_bytes());
        Self { buf }
    }

    pub fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn write_bytes(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    pub fn write_str(&mut self, s: &str) {
        self.write_u32(s.len() as u32);
        self.buf.extend_from_slice(s.as_bytes());
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}

pub struct SnapshotReader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> SnapshotReader<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self, KernelError> {
        if data.len() < 8 {
            return Err(KernelError::InvalidArgument);
        }
        if &data[0..4] != SNAPSHOT_MAGIC {
            return Err(KernelError::InvalidArgument);
        }
        let version = u32::from_le_bytes(data[4..8].try_into().map_err(|_| KernelError::InvalidArgument)?);
        if version != SNAPSHOT_VERSION {
            return Err(KernelError::InvalidArgument);
        }
        Ok(Self { data, offset: 8 })
    }

    pub fn read_u32(&mut self) -> Result<u32, KernelError> {
        if self.offset + 4 > self.data.len() {
            return Err(KernelError::InvalidArgument);
        }
        let v = u32::from_le_bytes(
            self.data[self.offset..self.offset + 4].try_into().map_err(|_| KernelError::InvalidArgument)?
        );
        self.offset += 4;
        Ok(v)
    }

    pub fn read_u64(&mut self) -> Result<u64, KernelError> {
        if self.offset + 8 > self.data.len() {
            return Err(KernelError::InvalidArgument);
        }
        let v = u64::from_le_bytes(
            self.data[self.offset..self.offset + 8].try_into().map_err(|_| KernelError::InvalidArgument)?
        );
        self.offset += 8;
        Ok(v)
    }

    pub fn read_u16(&mut self) -> Result<u16, KernelError> {
        if self.offset + 2 > self.data.len() {
            return Err(KernelError::InvalidArgument);
        }
        let v = u16::from_le_bytes(
            self.data[self.offset..self.offset + 2].try_into().map_err(|_| KernelError::InvalidArgument)?
        );
        self.offset += 2;
        Ok(v)
    }

    pub fn read_u8(&mut self) -> Result<u8, KernelError> {
        if self.offset + 1 > self.data.len() {
            return Err(KernelError::InvalidArgument);
        }
        let v = self.data[self.offset];
        self.offset += 1;
        Ok(v)
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], KernelError> {
        if self.offset + len > self.data.len() {
            return Err(KernelError::InvalidArgument);
        }
        let slice = &self.data[self.offset..self.offset + len];
        self.offset += len;
        Ok(slice)
    }

    pub fn read_string(&mut self) -> Result<alloc::string::String, KernelError> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        alloc::string::String::from_utf8(bytes.to_vec()).map_err(|_| KernelError::InvalidUtf8)
    }
}

impl Kernel {
    pub fn serialize(&self) -> Vec<u8> {
        let mut w = SnapshotWriter::new();

        // 1. CSpace slots
        {
            let slots: Vec<_> = self.cspace.iter().collect();
            w.write_u32(slots.len() as u32);
            for (slot, cap) in &slots {
                w.write_u32(**slot);
                w.write_u16(cap.rights.bits());
                w.write_u8(cap.object_type as u8);
                w.write_u32(cap.object_id);
                w.write_u64(cap.revoke_counter);
                w.write_u64(cap.epoch);
            }
        }

        // 2. Revoke map
        {
            let revoke_entries: Vec<_> = self.cspace.revoke_map_iter().collect();
            w.write_u32(revoke_entries.len() as u32);
            for (key, counter) in &revoke_entries {
                w.write_u8(key.type_ as u8);
                w.write_u32(key.id);
                w.write_u64(**counter);
            }
        }

        // 3. Next slot
        w.write_u32(self.cspace.next_slot());

        // 4. Inodes
        {
            let inodes: Vec<_> = self.inode_map.iter().collect();
            w.write_u32(inodes.len() as u32);
            for (_, inode) in &inodes {
                w.write_u32(inode.id);
                w.write_u8(match inode.inode_type {
                    InodeType::File => 0,
                    InodeType::Directory => 1,
                });
                w.write_str(&inode.name);
                w.write_u32(inode.data.len() as u32);
                w.write_bytes(&inode.data);
                w.write_u32(inode.children.len() as u32);
                for (name, child_id) in &inode.children {
                    w.write_str(name);
                    w.write_u32(*child_id);
                }
                w.write_u32(inode.parent.unwrap_or(u32::MAX));
                w.write_u64(inode.revoke_counter);
            }
        }

        // 5. Next inode ID, root inode ID
        w.write_u32(self.next_inode_id);
        w.write_u32(self.root_inode_id);

        // 6. WASI table
        {
            let active: Vec<_> = self.wasi_table.entries.iter().enumerate()
                .filter_map(|(fd, e)| e.as_ref().map(|e| (fd as u32, e)))
                .collect();
            w.write_u32(active.len() as u32);
            for (fd, entry) in &active {
                w.write_u32(*fd);
                w.write_u32(entry.cap_slot);
                w.write_u16(entry.rights.bits());
                w.write_u64(entry.offset);
                w.write_u8(entry.object_type as u8);
                w.write_u32(entry.object_id);
                w.write_u8(if entry.is_preopened { 1 } else { 0 });
            }
        }

        // 7. Delegations
        {
            let dels: Vec<_> = self.delegations.iter().collect();
            w.write_u32(dels.len() as u32);
            for (id, slot) in &dels {
                w.write_u64(**id);
                w.write_u32(**slot);
            }
        }

        // 8. Remote proxies
        {
            let proxies: Vec<_> = self.remote_proxies.iter().collect();
            w.write_u32(proxies.len() as u32);
            for (slot, proxy) in &proxies {
                w.write_u32(**slot);
                w.write_u64(proxy.peer_id);
                w.write_u64(proxy.delegation_id);
            }
        }

        // 9. Kernel metadata
        w.write_u64(self.start_time_ms);
        w.write_u32(self.next_pid);
        w.write_u32(self.current_pid);
        w.write_u32(self.root_cap_slot);
        w.write_u64(self.next_delegation_id);
        w.write_u64(self.kernel_id);

        w.into_vec()
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, KernelError> {
        let mut r = SnapshotReader::new(data)?;

        // 1. CSpace
        let n_slots = r.read_u32()?;
        let mut cspace = crate::CSpace::new();
        for _ in 0..n_slots {
            let slot = r.read_u32()?;
            let rights_bits = r.read_u16()?;
            let obj_type_u8 = r.read_u8()?;
            let obj_id = r.read_u32()?;
            let rev_cnt = r.read_u64()?;
            let epoch = r.read_u64()?;
            let obj_type = ObjectType::from_u8(obj_type_u8).ok_or(KernelError::InvalidArgument)?;
            cspace.restore_slot(slot, Capability {
                slot,
                object_type: obj_type,
                object_id: obj_id,
                rights: CapRights::new(rights_bits),
                revoke_counter: rev_cnt,
                epoch,
            });
        }

        // 2. Revoke map
        let n_revoke = r.read_u32()?;
        for _ in 0..n_revoke {
            let type_u8 = r.read_u8()?;
            let obj_type = ObjectType::from_u8(type_u8).ok_or(KernelError::InvalidArgument)?;
            let id = r.read_u32()?;
            let counter = r.read_u64()?;
            cspace.restore_revoke_entry(ObjectKey { type_: obj_type, id }, counter);
        }

        // 3. Next slot
        let next_slot = r.read_u32()?;
        cspace.set_next_slot(next_slot);

        // 4. Inodes
        let n_inodes = r.read_u32()?;
        let mut inode_map = BTreeMap::new();
        for _ in 0..n_inodes {
            let id = r.read_u32()?;
            let inode_type_u8 = r.read_u8()?;
            let inode_type = if inode_type_u8 == 0 { InodeType::File } else { InodeType::Directory };
            let name = r.read_string()?;
            let data_len = r.read_u32()? as usize;
            let data = r.read_bytes(data_len)?.to_vec();
            let n_children = r.read_u32()?;
            let mut children = BTreeMap::new();
            for _ in 0..n_children {
                let child_name = r.read_string()?;
                let child_id = r.read_u32()?;
                children.insert(child_name, child_id);
            }
            let parent_raw = r.read_u32()?;
            let parent = if parent_raw == u32::MAX { None } else { Some(parent_raw) };
            let revoke_counter = r.read_u64()?;
            inode_map.insert(id, Inode {
                id,
                inode_type,
                name,
                data,
                children,
                parent,
                revoke_counter,
            });
        }
        let next_inode_id = r.read_u32()?;
        let root_inode_id = r.read_u32()?;

        // 5. WASI table
        let mut wasi_table = crate::wasi::WasiTable::new();
        let n_wasi = r.read_u32()?;
        for _ in 0..n_wasi {
            let fd = r.read_u32()?;
            let cap_slot = r.read_u32()?;
            let rights_bits = r.read_u16()?;
            let offset = r.read_u64()?;
            let obj_type_u8 = r.read_u8()?;
            let obj_type = ObjectType::from_u8(obj_type_u8).ok_or(KernelError::InvalidArgument)?;
            let obj_id = r.read_u32()?;
            let is_preopened = r.read_u8()? != 0;
            if let Some(entry) = wasi_table.entries.get_mut(fd as usize) {
                *entry = Some(crate::wasi::WasiFdEntry {
                    cap_slot,
                    rights: CapRights::new(rights_bits),
                    offset,
                    object_type: obj_type,
                    object_id: obj_id,
                    is_preopened,
                });
            }
        }

        // 6. Delegations
        let n_dels = r.read_u32()?;
        let mut delegations = BTreeMap::new();
        for _ in 0..n_dels {
            let id = r.read_u64()?;
            let slot = r.read_u32()?;
            delegations.insert(id, slot);
        }

        // 7. Remote proxies
        let n_proxies = r.read_u32()?;
        let mut remote_proxies = BTreeMap::new();
        for _ in 0..n_proxies {
            let slot = r.read_u32()?;
            let peer_id = r.read_u64()?;
            let delegation_id = r.read_u64()?;
            remote_proxies.insert(slot, crate::distributed::RemoteProxyInfo { peer_id, delegation_id });
        }

        // 8. Kernel metadata
        let start_time_ms = r.read_u64()?;
        let next_pid = r.read_u32()?;
        let current_pid = r.read_u32()?;
        let root_cap_slot = r.read_u32()?;
        let next_delegation_id = r.read_u64()?;
        let kernel_id = r.read_u64()?;

        let kernel = Kernel {
            booted: true,
            start_time_ms,
            current_time_ms: start_time_ms,
            process_table: BTreeMap::new(),
            next_pid,
            current_pid,
            inode_map,
            next_inode_id,
            root_inode_id,
            open_files: BTreeMap::new(),
            next_fd: 3,
            cspace,
            root_cap_slot,
            wasi_table,
            delegations,
            remote_proxies,
            next_delegation_id,
            kernel_id,
            seen_nonces: BTreeSet::new(),
            next_nonce: 1,
        };

        Ok(kernel)
    }
}

impl Kernel {
    pub fn flush_all(&mut self) {
    }
}
