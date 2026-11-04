//! Hashing and generation of the secrets the server hands out.

use std::fmt::Write as _;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore as _;
use sha2::{Digest, Sha256};

/// Hex `SHA-256(pepper || value)`: how codes, tokens and links are stored.
pub fn secret_hash(pepper: &[u8], value: &str) -> String {
    let digest = Sha256::new()
        .chain_update(pepper)
        .chain_update(value.as_bytes())
        .finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// `prefix` followed by 32 random bytes, base64url without padding.
pub fn new_token(prefix: &str) -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let mut token = String::with_capacity(prefix.len() + 43);
    token.push_str(prefix);
    URL_SAFE_NO_PAD.encode_string(bytes, &mut token);
    token
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_hash_is_hex_sha256_of_pepper_then_value() {
        // printf 'pepperabc' | shasum -a 256
        assert_eq!(
            secret_hash(b"pepper", "abc"),
            "fadf7b97406e1eaa259a82e26e6839e564c37c256876e060e17838c86b7275ef"
        );
    }

    #[test]
    fn secret_hash_depends_on_pepper() {
        assert_ne!(secret_hash(b"a", "value"), secret_hash(b"b", "value"));
    }

    #[test]
    fn new_token_is_prefix_plus_43_url_safe_chars() {
        let token = new_token("crs_");
        let rest = token.strip_prefix("crs_").unwrap();
        assert_eq!(rest.len(), 43);
        assert!(
            rest.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        );
        assert_eq!(URL_SAFE_NO_PAD.decode(rest).unwrap().len(), 32);
    }

    #[test]
    fn new_token_is_random() {
        assert_ne!(new_token(""), new_token(""));
    }
}
