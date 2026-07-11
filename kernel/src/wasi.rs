use alloc::vec::Vec;
use crate::{CapRights, ObjectType};

const WASI_FD_MAX: usize = 256;

#[derive(Debug, Clone)]
pub struct WasiFdEntry {
    pub cap_slot: u32,
    pub rights: CapRights,
    pub offset: u64,
    pub object_type: ObjectType,
    pub object_id: u32,
    pub is_preopened: bool,
}

#[derive(Debug, Clone)]
pub struct WasiTable {
    pub entries: Vec<Option<WasiFdEntry>>,
}

impl WasiTable {
    pub fn new() -> Self {
        let mut entries = Vec::new();
        entries.resize_with(WASI_FD_MAX, || None);
        Self { entries }
    }

    pub fn init_root(&mut self, root_cap_slot: u32, root_inode_id: u32) {
        self.entries[3] = Some(WasiFdEntry {
            cap_slot: root_cap_slot,
            rights: CapRights::new(CapRights::READ | CapRights::WRITE | CapRights::GRANT | CapRights::EXECUTE),
            offset: 0,
            object_type: ObjectType::Inode,
            object_id: root_inode_id,
            is_preopened: true,
        });
    }

    pub fn alloc(&mut self, entry: WasiFdEntry) -> Option<u32> {
        for fd in 4..WASI_FD_MAX {
            if self.entries[fd].is_none() {
                self.entries[fd] = Some(entry);
                return Some(fd as u32);
            }
        }
        None
    }

    pub fn get(&self, fd: u32) -> Option<&WasiFdEntry> {
        self.entries.get(fd as usize).and_then(|e| e.as_ref())
    }

    pub fn get_mut(&mut self, fd: u32) -> Option<&mut WasiFdEntry> {
        self.entries.get_mut(fd as usize).and_then(|e| e.as_mut())
    }

    pub fn free(&mut self, fd: u32) {
        if let Some(entry) = self.entries.get_mut(fd as usize) {
            *entry = None;
        }
    }

    pub fn alloc_from_slot(&mut self, cap_slot: u32, object_type: ObjectType, object_id: u32, rights: CapRights) -> Option<u32> {
        self.alloc(WasiFdEntry {
            cap_slot,
            rights,
            offset: 0,
            object_type,
            object_id,
            is_preopened: false,
        })
    }

    pub fn active_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasi_table_basic() {
        let mut table = WasiTable::new();
        assert_eq!(table.active_count(), 0);

        table.init_root(1, 0);
        assert!(table.get(3).is_some());
        assert_eq!(table.get(3).unwrap().cap_slot, 1);

        let fd = table.alloc_from_slot(2, ObjectType::Inode, 5, CapRights::new(CapRights::READ)).unwrap();
        assert!(fd >= 4);
        assert_eq!(table.get(fd).unwrap().cap_slot, 2);

        table.free(fd);
        assert!(table.get(fd).is_none());
    }

    #[test]
    fn test_wasi_table_exhaustion() {
        let mut table = WasiTable::new();
        table.init_root(1, 0);
        for _ in 0..252 {
            table.alloc_from_slot(2, ObjectType::Inode, 5, CapRights::new(CapRights::READ));
        }
        assert!(table.alloc_from_slot(3, ObjectType::Inode, 6, CapRights::new(CapRights::READ)).is_none());
    }
}
