use uuid::Uuid;

use crate::keypair::{verify, Keypair};

const PAYLOAD_LEN: usize = 16 + 16 + 32 + 8; // agent_id + account_id + fingerprint + issued_at
const SIGNATURE_LEN: usize = 64;
const CREDENTIAL_LEN: usize = PAYLOAD_LEN + SIGNATURE_LEN;

/// The data a long-lived agent credential attests to. Binds an agent
/// identity to an account and to the fingerprint of the public key it was
/// issued for, per the locked-in security model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialPayload {
    pub agent_id: Uuid,
    pub account_id: Uuid,
    pub public_key_fingerprint: [u8; 32],
    /// Unix seconds.
    pub issued_at: i64,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CredentialError {
    #[error("credential has the wrong length")]
    Malformed,
    #[error("credential signature is invalid")]
    InvalidSignature,
}

impl CredentialPayload {
    fn to_bytes(&self) -> [u8; PAYLOAD_LEN] {
        let mut buf = [0u8; PAYLOAD_LEN];
        buf[0..16].copy_from_slice(self.agent_id.as_bytes());
        buf[16..32].copy_from_slice(self.account_id.as_bytes());
        buf[32..64].copy_from_slice(&self.public_key_fingerprint);
        buf[64..72].copy_from_slice(&self.issued_at.to_le_bytes());
        buf
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, CredentialError> {
        if bytes.len() != PAYLOAD_LEN {
            return Err(CredentialError::Malformed);
        }
        let agent_id = Uuid::from_slice(&bytes[0..16]).map_err(|_| CredentialError::Malformed)?;
        let account_id =
            Uuid::from_slice(&bytes[16..32]).map_err(|_| CredentialError::Malformed)?;
        let mut public_key_fingerprint = [0u8; 32];
        public_key_fingerprint.copy_from_slice(&bytes[32..64]);
        let mut issued_at_bytes = [0u8; 8];
        issued_at_bytes.copy_from_slice(&bytes[64..72]);
        let issued_at = i64::from_le_bytes(issued_at_bytes);

        Ok(Self {
            agent_id,
            account_id,
            public_key_fingerprint,
            issued_at,
        })
    }
}

/// Sign a credential payload with the control plane's keypair, producing
/// the opaque bytes handed to the agent (and later presented back).
pub fn sign_credential(payload: &CredentialPayload, signer: &Keypair) -> Vec<u8> {
    let payload_bytes = payload.to_bytes();
    let signature = signer.sign(&payload_bytes);

    let mut out = Vec::with_capacity(CREDENTIAL_LEN);
    out.extend_from_slice(&payload_bytes);
    out.extend_from_slice(&signature);
    out
}

/// Verify a credential was signed by the holder of `control_plane_public_key`
/// and, if so, return the payload it attests to. Does not check expiry or
/// revocation — those require a database lookup and are the caller's job.
pub fn verify_credential(
    bytes: &[u8],
    control_plane_public_key: &[u8; 32],
) -> Result<CredentialPayload, CredentialError> {
    if bytes.len() != CREDENTIAL_LEN {
        return Err(CredentialError::Malformed);
    }
    let (payload_bytes, signature_bytes) = bytes.split_at(PAYLOAD_LEN);
    let mut signature = [0u8; SIGNATURE_LEN];
    signature.copy_from_slice(signature_bytes);

    if !verify(control_plane_public_key, payload_bytes, &signature) {
        return Err(CredentialError::InvalidSignature);
    }

    CredentialPayload::from_bytes(payload_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> CredentialPayload {
        CredentialPayload {
            agent_id: Uuid::new_v4(),
            account_id: Uuid::new_v4(),
            public_key_fingerprint: [7u8; 32],
            issued_at: 1_700_000_000,
        }
    }

    #[test]
    fn sign_then_verify_roundtrip() {
        let control_plane = Keypair::generate();
        let payload = sample_payload();

        let credential = sign_credential(&payload, &control_plane);
        let verified = verify_credential(&credential, &control_plane.public_key_bytes()).unwrap();

        assert_eq!(verified, payload);
    }

    #[test]
    fn verify_rejects_wrong_signer() {
        let control_plane = Keypair::generate();
        let impostor = Keypair::generate();
        let payload = sample_payload();

        let credential = sign_credential(&payload, &impostor);
        let result = verify_credential(&credential, &control_plane.public_key_bytes());

        assert_eq!(result, Err(CredentialError::InvalidSignature));
    }

    #[test]
    fn verify_rejects_tampered_payload() {
        let control_plane = Keypair::generate();
        let payload = sample_payload();

        let mut credential = sign_credential(&payload, &control_plane);
        credential[0] ^= 0xFF; // flip a byte inside agent_id

        let result = verify_credential(&credential, &control_plane.public_key_bytes());
        assert_eq!(result, Err(CredentialError::InvalidSignature));
    }

    #[test]
    fn verify_rejects_malformed_length() {
        let control_plane = Keypair::generate();
        let result = verify_credential(&[1, 2, 3], &control_plane.public_key_bytes());
        assert_eq!(result, Err(CredentialError::Malformed));
    }
}
