use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use signature::Verifier;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("invalid public key hex: {0}")]
    InvalidPublicKey(String),
    #[error("invalid signature base64: {0}")]
    InvalidSignature(String),
    #[error("signature verification failed")]
    VerificationFailed,
}

/// Verify an ed25519 signature over `artifact_bytes`.
///
/// `public_key_hex` — 32-byte ed25519 verifying key encoded as lowercase hex (64 chars).
/// `signature_base64` — 64-byte ed25519 signature encoded as standard base64.
pub fn verify_capability(
    artifact_bytes: &[u8],
    public_key_hex: &str,
    signature_base64: &str,
) -> Result<(), VerifyError> {
    // Decode public key.
    let pk_bytes =
        hex::decode(public_key_hex).map_err(|e| VerifyError::InvalidPublicKey(e.to_string()))?;
    let pk_arr: [u8; 32] = pk_bytes
        .try_into()
        .map_err(|_| VerifyError::InvalidPublicKey("expected 32 bytes".into()))?;
    let verifying_key = VerifyingKey::from_bytes(&pk_arr)
        .map_err(|e| VerifyError::InvalidPublicKey(e.to_string()))?;

    // Decode signature.
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_base64)
        .map_err(|e| VerifyError::InvalidSignature(e.to_string()))?;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| VerifyError::InvalidSignature("expected 64 bytes".into()))?;
    let signature = Signature::from_bytes(&sig_arr);

    // Verify.
    verifying_key
        .verify(artifact_bytes, &signature)
        .map_err(|_| VerifyError::VerificationFailed)
}
