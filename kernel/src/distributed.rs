use alloc::vec::Vec;
use crate::{CapRights, ObjectType, KernelError, Capability};
use crate::crypto::{hmac_sha256, constant_time_eq};
use crate::Kernel;

#[derive(Debug, Clone)]
pub struct DelegationToken {
    pub delegation_id: u64,
    pub rights: CapRights,
    pub object_type: ObjectType,
    pub object_id: u32,
    pub origin_kernel_id: u64,
    pub nonce: u64,
    pub hmac_tag: [u8; 32],
}

impl DelegationToken {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(63);
        buf.extend_from_slice(&self.delegation_id.to_le_bytes());
        buf.extend_from_slice(&self.rights.bits().to_le_bytes());
        buf.push(self.object_type as u8);
        buf.extend_from_slice(&self.object_id.to_le_bytes());
        buf.extend_from_slice(&self.origin_kernel_id.to_le_bytes());
        buf.extend_from_slice(&self.nonce.to_le_bytes());
        buf.extend_from_slice(&self.hmac_tag);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KernelError> {
        if bytes.len() < 63 {
            return Err(KernelError::InvalidArgument);
        }
        let mut off = 0;
        let delegation_id = u64::from_le_bytes(bytes[off..off+8].try_into().map_err(|_| KernelError::InvalidArgument)?); off += 8;
        let rights_bits = u16::from_le_bytes(bytes[off..off+2].try_into().map_err(|_| KernelError::InvalidArgument)?); off += 2;
        let object_type = ObjectType::from_u8(bytes[off]).ok_or(KernelError::InvalidArgument)?; off += 1;
        let object_id = u32::from_le_bytes(bytes[off..off+4].try_into().map_err(|_| KernelError::InvalidArgument)?); off += 4;
        let origin_kernel_id = u64::from_le_bytes(bytes[off..off+8].try_into().map_err(|_| KernelError::InvalidArgument)?); off += 8;
        let nonce = u64::from_le_bytes(bytes[off..off+8].try_into().map_err(|_| KernelError::InvalidArgument)?); off += 8;
        let mut hmac_tag = [0u8; 32];
        hmac_tag.copy_from_slice(&bytes[off..off+32]);
        Ok(Self {
            delegation_id,
            rights: CapRights::new(rights_bits),
            object_type,
            object_id,
            origin_kernel_id,
            nonce,
            hmac_tag,
        })
    }

    fn payload_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(37);
        buf.extend_from_slice(&self.delegation_id.to_le_bytes());
        buf.extend_from_slice(&self.rights.bits().to_le_bytes());
        buf.push(self.object_type as u8);
        buf.extend_from_slice(&self.object_id.to_le_bytes());
        buf.extend_from_slice(&self.origin_kernel_id.to_le_bytes());
        buf.extend_from_slice(&self.nonce.to_le_bytes());
        buf
    }
}

pub struct PeerKey([u8; 32]);

impl PeerKey {
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }

    pub fn from_oob_secret(secret: &[u8]) -> Self {
        let hash = crate::crypto::sha256(secret);
        Self(hash)
    }

    pub fn compute_hmac(&self, data: &[u8]) -> [u8; 32] {
        hmac_sha256(&self.0, data)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct RemoteProxyInfo {
    pub peer_id: u64,
    pub delegation_id: u64,
}

impl Kernel {
    pub fn delegate_cap(
        &mut self,
        slot: u32,
        peer_key: &PeerKey,
    ) -> Result<DelegationToken, KernelError> {
        let (object_type, object_id, rights) = {
            let cap = self.check_cap_raw(slot, CapRights::new(CapRights::GRANT))?;
            #[cfg(test)]
            extern crate std;
            #[cfg(test)]
            std::println!("DELEGATE: slot={}, rights={:?}, nonce_start={}",
                slot, cap.rights.bits(), self.next_nonce);
            (cap.object_type, cap.object_id, cap.rights)
        };

        let delegation_id = self.next_delegation_id;
        self.next_delegation_id = self.next_delegation_id.wrapping_add(1);

        let nonce = self.generate_nonce();

        let token = DelegationToken {
            delegation_id,
            rights,
            object_type,
            object_id,
            origin_kernel_id: self.kernel_id,
            nonce,
            hmac_tag: [0u8; 32],
        };

        let payload = token.payload_bytes();
        let hmac = peer_key.compute_hmac(&payload);
        let mut final_token = token;
        final_token.hmac_tag = hmac;

        self.delegations.insert(delegation_id, slot);
        Ok(final_token)
    }

    pub fn import_delegation(
        &mut self,
        token: &DelegationToken,
        peer_key: &PeerKey,
        peer_id: u64,
    ) -> Result<u32, KernelError> {
        let payload = token.payload_bytes();
        let expected_hmac = peer_key.compute_hmac(&payload);
        #[cfg(test)]
        {
            extern crate std;
            let matches = constant_time_eq(&expected_hmac, &token.hmac_tag);
            std::println!("IMPORT: tag_matches={}, nonce={}", matches, token.nonce);
            if !matches {
                std::println!("  expected_hmac={:02x?}", expected_hmac);
                std::println!("  token.hmac   ={:02x?}", token.hmac_tag);
                std::println!("  payload={:02x?}", payload);
            }
        }
        if !constant_time_eq(&expected_hmac, &token.hmac_tag) {
            return Err(KernelError::AccessDenied);
        }

        if self.seen_nonces.contains(&token.nonce) {
            return Err(KernelError::AccessDenied);
        }
        self.seen_nonces.insert(token.nonce);

        let slot = self.cspace.allocate_slot();
        self.cspace.install(slot, Capability {
            slot,
            object_type: ObjectType::RemoteProxy,
            object_id: slot,
            rights: token.rights,
            revoke_counter: 0,
            epoch: self.cspace.global_epoch,
        });

        self.remote_proxies.insert(slot, RemoteProxyInfo {
            peer_id,
            delegation_id: token.delegation_id,
        });

        Ok(slot)
    }

    pub fn revoke_delegation(&mut self, delegation_id: u64) -> Result<(), KernelError> {
        let slot = self.delegations.remove(&delegation_id)
            .ok_or(KernelError::InvalidArgument)?;
        if let Some(cap) = self.cspace.get_mut(slot) {
            cap.revoke_counter = cap.revoke_counter.wrapping_add(1);
        }
        Ok(())
    }

    pub fn list_remote_proxies(&self) -> Vec<(u64, u64, u64)> {
        self.remote_proxies.iter()
            .map(|(slot, info)| (*slot as u64, info.peer_id, info.delegation_id))
            .collect()
    }

    pub fn list_delegations(&self) -> Vec<(u64, u32)> {
        self.delegations.iter()
            .map(|(id, slot)| (*id, *slot))
            .collect()
    }

    pub fn generate_nonce(&mut self) -> u64 {
        let nonce = self.next_nonce;
        self.next_nonce = self.next_nonce.wrapping_add(1);
        nonce
    }
}

#[derive(Debug, Clone)]
pub struct RemotableObjectRef {
    pub kernel_id: u64,
    pub delegation_id: u64,
    pub rights: CapRights,
    pub object_type: ObjectType,
    pub object_id: u32,
}

impl RemotableObjectRef {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(30);
        buf.extend_from_slice(&self.kernel_id.to_le_bytes());
        buf.extend_from_slice(&self.delegation_id.to_le_bytes());
        buf.extend_from_slice(&self.rights.bits().to_le_bytes());
        buf.push(self.object_type as u8);
        buf.extend_from_slice(&self.object_id.to_le_bytes());
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KernelError> {
        if bytes.len() < 30 {
            return Err(KernelError::InvalidArgument);
        }
        let mut off = 0;
        let kernel_id = u64::from_le_bytes(bytes[off..off+8].try_into().map_err(|_| KernelError::InvalidArgument)?); off += 8;
        let delegation_id = u64::from_le_bytes(bytes[off..off+8].try_into().map_err(|_| KernelError::InvalidArgument)?); off += 8;
        let rights_bits = u16::from_le_bytes(bytes[off..off+2].try_into().map_err(|_| KernelError::InvalidArgument)?); off += 2;
        let object_type = ObjectType::from_u8(bytes[off]).ok_or(KernelError::InvalidArgument)?; off += 1;
        let object_id = u32::from_le_bytes(bytes[off..off+4].try_into().map_err(|_| KernelError::InvalidArgument)?);
        Ok(Self { kernel_id, delegation_id, rights: CapRights::new(rights_bits), object_type, object_id })
    }
}

impl From<DelegationToken> for alloc::vec::Vec<u8> {
    fn from(token: DelegationToken) -> Self {
        token.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests;

    #[test]
    fn test_delegate_and_import() {
        let _kg = tests::setup_kernel();
        tests::with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let peer_key = PeerKey::from_oob_secret(b"test-shared-secret");
            let token = k.delegate_cap(root_slot, &peer_key).unwrap();
            assert_eq!(token.rights.bits(), CapRights::ALL);
            let slot = k.import_delegation(&token, &peer_key, 42).unwrap();
            assert!(slot > 0);
        });
    }

    #[test]
    fn test_delegate_rejects_no_grant() {
        let _kg = tests::setup_kernel();
        tests::with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let read_only = k.cap_mint(root_slot, ObjectType::Inode, 0,
                CapRights::new(CapRights::READ)).unwrap();
            let peer_key = PeerKey::from_oob_secret(b"test");
            let result = k.delegate_cap(read_only, &peer_key);
            assert!(matches!(result, Err(KernelError::InsufficientRights { .. })));
        });
    }

    #[test]
    fn test_import_rejects_forged_hmac() {
        let _kg = tests::setup_kernel();
        tests::with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let peer_key = PeerKey::from_oob_secret(b"real-key");
            let mut token = k.delegate_cap(root_slot, &peer_key).unwrap();
            token.hmac_tag[0] ^= 0xFF;
            let wrong_key = PeerKey::from_oob_secret(b"wrong-key");
            let result = k.import_delegation(&token, &wrong_key, 42);
            assert!(matches!(result, Err(KernelError::AccessDenied)));
        });
    }

    #[test]
    fn test_import_rejects_replay() {
        let _kg = tests::setup_kernel();
        tests::with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let peer_key = PeerKey::from_oob_secret(b"test");
            let token = k.delegate_cap(root_slot, &peer_key).unwrap();
            let r1 = k.import_delegation(&token, &peer_key, 1);
            assert!(r1.is_ok());
            let r2 = k.import_delegation(&token, &peer_key, 2);
            assert!(matches!(r2, Err(KernelError::AccessDenied)));
        });
    }

    #[test]
    fn test_delegation_token_roundtrip() {
        let _kg = tests::setup_kernel();
        tests::with_kernel_guarded(|k| {
            let root_slot = k.root_cap_slot;
            let peer_key = PeerKey::from_oob_secret(b"roundtrip");
            let token = k.delegate_cap(root_slot, &peer_key).unwrap();
            let bytes = token.to_bytes();
            let parsed = DelegationToken::from_bytes(&bytes).unwrap();
            assert_eq!(parsed.delegation_id, token.delegation_id);
            assert_eq!(parsed.rights.bits(), token.rights.bits());
            assert_eq!(parsed.object_type, token.object_type);
            assert_eq!(parsed.object_id, token.object_id);
            assert_eq!(parsed.origin_kernel_id, token.origin_kernel_id);
            assert_eq!(parsed.nonce, token.nonce);
        });
    }
}
