use sha2::{Digest, Sha256};

/// A public key fingerprint: SHA-256 of the raw 32-byte Ed25519 public key.
/// Used for compact identification/logging and for the mismatch check in
/// the security model (credential vs. stored public key for a known agent).
pub fn fingerprint(public_key: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    hasher.finalize().into()
}

pub fn to_hex(fingerprint: &[u8; 32]) -> String {
    fingerprint.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keypair::Keypair;

    #[test]
    fn same_key_same_fingerprint() {
        let kp = Keypair::generate();
        let pk = kp.public_key_bytes();
        assert_eq!(fingerprint(&pk), fingerprint(&pk));
    }

    #[test]
    fn different_keys_different_fingerprints() {
        let a = Keypair::generate().public_key_bytes();
        let b = Keypair::generate().public_key_bytes();
        assert_ne!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn hex_is_64_chars() {
        let kp = Keypair::generate();
        let hex = to_hex(&fingerprint(&kp.public_key_bytes()));
        assert_eq!(hex.len(), 64);
    }
}
