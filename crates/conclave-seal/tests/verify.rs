use conclave_seal::verify_capability;
use ed25519_dalek::{Signer, SigningKey};

fn gen_key_and_sign(artifact: &[u8]) -> (String, String) {
    // Generate a deterministic signing key from a fixed seed for testing.
    let seed: [u8; 32] = [0x42u8; 32];
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    let signature = signing_key.sign(artifact);

    let pk_hex = hex::encode(verifying_key.as_bytes());
    let sig_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        signature.to_bytes(),
    );
    (pk_hex, sig_b64)
}

#[test]
fn verify_valid_signature() {
    let artifact = b"#!/bin/sh\necho hello";
    let (pk_hex, sig_b64) = gen_key_and_sign(artifact);
    verify_capability(artifact, &pk_hex, &sig_b64).expect("valid signature should verify");
}

#[test]
fn verify_rejects_tampered_artifact() {
    let artifact = b"#!/bin/sh\necho hello";
    let (pk_hex, sig_b64) = gen_key_and_sign(artifact);
    let tampered = b"#!/bin/sh\necho evil";
    let result = verify_capability(tampered, &pk_hex, &sig_b64);
    assert!(
        result.is_err(),
        "tampered artifact should fail verification"
    );
}

#[test]
fn verify_rejects_wrong_public_key() {
    let artifact = b"capability bytes";
    let (_, sig_b64) = gen_key_and_sign(artifact);
    // Use a different key for verification.
    let seed2: [u8; 32] = [0x99u8; 32];
    let other_key = ed25519_dalek::SigningKey::from_bytes(&seed2);
    let wrong_pk = hex::encode(other_key.verifying_key().as_bytes());
    let result = verify_capability(artifact, &wrong_pk, &sig_b64);
    assert!(result.is_err(), "wrong public key should fail verification");
}

#[test]
fn verify_rejects_invalid_public_key_hex() {
    let result = verify_capability(b"bytes", "not-hex", "AAAA");
    assert!(result.is_err());
}

#[test]
fn verify_rejects_invalid_signature_base64() {
    let seed: [u8; 32] = [0x42u8; 32];
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
    let pk_hex = hex::encode(signing_key.verifying_key().as_bytes());
    let result = verify_capability(b"bytes", &pk_hex, "not-valid-base64!!!");
    assert!(result.is_err());
}
