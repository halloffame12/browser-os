use crate::{CapRights, KernelError};
use crate::with_kernel_mut;
use core::marker::PhantomData;

pub struct Unverified;
pub struct ReadOnly;
pub struct WriteOnly;
pub struct ReadWrite;

mod sealed {
    pub trait Sealed {}
}
impl sealed::Sealed for ReadOnly {}
impl sealed::Sealed for WriteOnly {}
impl sealed::Sealed for ReadWrite {}

pub trait CanRead: sealed::Sealed {}
pub trait CanWrite: sealed::Sealed {}

impl CanRead for ReadOnly {}
impl CanRead for ReadWrite {}
impl CanWrite for WriteOnly {}
impl CanWrite for ReadWrite {}

pub struct FileDescriptor<State> {
    slot: u32,
    rights: CapRights,
    offset: u64,
    _phantom: PhantomData<State>,
}

impl<State> FileDescriptor<State> {
    pub fn slot(&self) -> u32 { self.slot }
    pub fn rights(&self) -> CapRights { self.rights }
    pub fn offset(&self) -> u64 { self.offset }
}

impl FileDescriptor<Unverified> {
    pub fn new(slot: u32, rights: CapRights) -> Self {
        Self { slot, rights, offset: 0, _phantom: PhantomData }
    }

    pub fn check_read_only(self) -> Result<FileDescriptor<ReadOnly>, KernelError> {
        if !self.rights.contains(CapRights::READ) {
            return Err(KernelError::InsufficientRights {
                cap: self.slot,
                missing: CapRights::new(CapRights::READ),
            });
        }
        Ok(FileDescriptor {
            slot: self.slot,
            rights: CapRights::new(CapRights::READ),
            offset: self.offset,
            _phantom: PhantomData,
        })
    }

    pub fn check_write_only(self) -> Result<FileDescriptor<WriteOnly>, KernelError> {
        if !self.rights.contains(CapRights::WRITE) {
            return Err(KernelError::InsufficientRights {
                cap: self.slot,
                missing: CapRights::new(CapRights::WRITE),
            });
        }
        Ok(FileDescriptor {
            slot: self.slot,
            rights: CapRights::new(CapRights::WRITE),
            offset: self.offset,
            _phantom: PhantomData,
        })
    }

    pub fn check_read_write(self) -> Result<FileDescriptor<ReadWrite>, KernelError> {
        if !self.rights.contains(CapRights::READ | CapRights::WRITE) {
            return Err(KernelError::InsufficientRights {
                cap: self.slot,
                missing: CapRights::new(CapRights::READ | CapRights::WRITE),
            });
        }
        Ok(FileDescriptor {
            slot: self.slot,
            rights: CapRights::new(CapRights::READ | CapRights::WRITE),
            offset: self.offset,
            _phantom: PhantomData,
        })
    }
}

impl<State: CanRead> FileDescriptor<State> {
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, KernelError> {
        with_kernel_mut(|k| {
            let n = k.cap_read_into_slice(self.slot, self.offset, buf)?;
            self.offset += n as u64;
            Ok(n)
        })
    }
}

impl<State: CanWrite> FileDescriptor<State> {
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, KernelError> {
        with_kernel_mut(|k| {
            let n = k.cap_write_from_slice(self.slot, self.offset, buf)?;
            self.offset += n as u64;
            Ok(n)
        })
    }
}

impl FileDescriptor<ReadWrite> {
    pub fn restrict_to_read(self) -> FileDescriptor<ReadOnly> {
        FileDescriptor {
            slot: self.slot,
            rights: CapRights::new(CapRights::READ),
            offset: self.offset,
            _phantom: PhantomData,
        }
    }

    pub fn restrict_to_write(self) -> FileDescriptor<WriteOnly> {
        FileDescriptor {
            slot: self.slot,
            rights: CapRights::new(CapRights::WRITE),
            offset: self.offset,
            _phantom: PhantomData,
        }
    }
}

impl<State> Drop for FileDescriptor<State> {
    fn drop(&mut self) {
        unsafe {
            let ptr = crate::KERNEL.0.get();
            if let Some(ref mut kernel) = *ptr {
                let _ = kernel.cap_destroy(self.slot);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests;

    #[test]
    fn test_typestate_read_write_roundtrip() {
        let _kg = tests::setup_kernel();
        tests::with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let file_cap = k.cap_create_file(root_slot, "/typestate_test.txt", false).unwrap() as u32;
            k.cap_write_file(file_cap, b"hello").unwrap();
            let rights = CapRights::new(CapRights::READ | CapRights::WRITE | CapRights::GRANT | CapRights::EXECUTE | CapRights::DELETE);
            let fd = FileDescriptor::<Unverified>::new(file_cap, rights);

            let mut rw_fd = fd.check_read_write().unwrap();
            let mut buf = [0u8; 5];
            let result = rw_fd.read(&mut buf);
            assert!(result.is_ok());
            assert_eq!(&buf, b"hello");
        });
    }

    #[test]
    fn test_typestate_attenuation_readwrite_to_readonly() {
        let _kg = tests::setup_kernel();
        tests::with_kernel_guarded(|k| {
            let slot = k.root_cap_slot;
            let rights = CapRights::new(CapRights::READ | CapRights::WRITE | CapRights::GRANT | CapRights::EXECUTE | CapRights::DELETE);
            let fd = FileDescriptor::<Unverified>::new(slot, rights);
            let rw_fd = fd.check_read_write().unwrap();
            let ro_fd = rw_fd.restrict_to_read();
            drop(ro_fd);
        });
    }
}
