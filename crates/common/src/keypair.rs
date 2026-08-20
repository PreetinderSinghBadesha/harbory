use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use std::io;
use std::path::Path;

/// An Ed25519 keypair. Used both by agents (identity) and by the control
/// plane (credential signing) — same primitive, different roles.
#[derive(Clone)]
pub struct Keypair {
    signing_key: SigningKey,
}

impl Keypair {
    pub fn generate() -> Self {
        Self {
            signing_key: SigningKey::generate(&mut OsRng),
        }
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(bytes),
        }
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing_key.sign(message).to_bytes()
    }

    /// Load a keypair from `path`, or generate and persist a new one if it
    /// doesn't exist yet. Used both for the agent's local identity and for
    /// the control plane's credential-signing key, so a restart doesn't
    /// invalidate previously issued credentials/identities.
    ///
    /// On Unix the file is created with `0600` permissions. Windows has no
    /// equivalent ACL-free mechanism, so this is best-effort there — deploy
    /// on Windows via a directory ACL instead if that matters.
    pub fn load_or_generate(path: &Path) -> io::Result<Self> {
        if path.exists() {
            let bytes = std::fs::read(path)?;
            let array: [u8; 32] = bytes
                .try_into()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "key file is not 32 bytes"))?;
            return Ok(Self::from_bytes(&array));
        }

        let keypair = Self::generate();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, keypair.to_bytes())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }

        Ok(keypair)
    }
}

/// Verify a signature against a raw 32-byte Ed25519 public key.
pub fn verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    let Ok(verifying_key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let signature = Signature::from_bytes(signature);
    verifying_key.verify(message, &signature).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let kp = Keypair::generate();
        let sig = kp.sign(b"hello");
        assert!(verify(&kp.public_key_bytes(), b"hello", &sig));
    }

    #[test]
    fn verify_rejects_tampered_message() {
        let kp = Keypair::generate();
        let sig = kp.sign(b"hello");
        assert!(!verify(&kp.public_key_bytes(), b"goodbye", &sig));
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let kp = Keypair::generate();
        let other = Keypair::generate();
        let sig = kp.sign(b"hello");
        assert!(!verify(&other.public_key_bytes(), b"hello", &sig));
    }

    #[test]
    fn keypair_bytes_roundtrip() {
        let kp = Keypair::generate();
        let bytes = kp.to_bytes();
        let restored = Keypair::from_bytes(&bytes);
        assert_eq!(kp.public_key_bytes(), restored.public_key_bytes());
    }
}
