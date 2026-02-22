use sha2::{Digest, Sha256};

/// A SHA-256 hash in "sha256:<hex>" format.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct Hash(String);

impl Hash {
    /// Parse a "sha256:<hex>" string.
    pub fn parse(s: &str) -> Result<Self, crate::HashError> {
        if let Some(hex_part) = s.strip_prefix("sha256:") {
            if hex_part.len() == 64 && hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
                return Ok(Hash(s.to_string()));
            }
        }
        Err(crate::HashError::InvalidFormat(s.to_string()))
    }

    /// Return the hex portion (64 characters).
    pub fn hex(&self) -> &str {
        &self.0["sha256:".len()..]
    }

    /// Hash raw bytes and return "sha256:<hex>".
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let result = hasher.finalize();
        Hash(format!("sha256:{}", hex::encode(result)))
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Hash raw bytes with SHA-256.
pub fn sha256_bytes(data: &[u8]) -> Hash {
    Hash::from_bytes(data)
}

/// Hash a UTF-8 string with SHA-256.
pub fn sha256_str(data: &str) -> Hash {
    Hash::from_bytes(data.as_bytes())
}
