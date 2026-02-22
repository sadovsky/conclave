use conclave_hash::{sha256_bytes, sha256_str, Hash};

#[test]
fn sha256_empty_string() {
    let h = sha256_str("");
    assert_eq!(
        h.to_string(),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn sha256_known_vector_abc() {
    // echo -n "abc" | sha256sum
    let h = sha256_str("abc");
    // NIST FIPS 180-4 SHA-256 test vector for "abc"
    assert_eq!(
        h.to_string(),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn sha256_bytes_matches_str() {
    let h1 = sha256_str("hello");
    let h2 = sha256_bytes(b"hello");
    assert_eq!(h1, h2);
}

#[test]
fn hash_parse_valid() {
    let s = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let h = Hash::parse(s).unwrap();
    assert_eq!(h.to_string(), s);
    assert_eq!(h.hex().len(), 64);
}

#[test]
fn hash_parse_invalid_prefix() {
    assert!(Hash::parse("md5:abc").is_err());
}

#[test]
fn hash_parse_invalid_hex_length() {
    assert!(Hash::parse("sha256:abc").is_err());
}

#[test]
fn hash_display_has_prefix() {
    let h = sha256_str("test");
    assert!(h.to_string().starts_with("sha256:"));
}
