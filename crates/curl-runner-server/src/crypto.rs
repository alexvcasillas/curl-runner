//! Reversible encryption for the one secret the server must show again: a
//! collection's share link token. XChaCha20-Poly1305 under a key derived from
//! `SECRET_KEY`; the stored form is base64url of `nonce(24) || ciphertext`.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use rand::RngCore as _;
use sha2::Sha256;

const NONCE_LEN: usize = 24;
const KEY_INFO: &[u8] = b"curl-runner link token v1";

fn cipher(secret_key: &[u8]) -> XChaCha20Poly1305 {
    let mut key = [0u8; 32];
    Hkdf::<Sha256>::new(None, secret_key)
        .expand(KEY_INFO, &mut key)
        .expect("32 bytes is a valid HKDF output length");
    XChaCha20Poly1305::new((&key).into())
}

/// Encrypt a link token for storage in `collections.link_token_enc`.
pub fn encrypt_link_token(secret_key: &[u8], token: &str) -> String {
    let mut nonce = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut nonce);
    let ciphertext = cipher(secret_key)
        .encrypt(XNonce::from_slice(&nonce), token.as_bytes())
        .expect("encrypting in memory cannot fail");
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    URL_SAFE_NO_PAD.encode(out)
}

/// The clear token, or `None` when `enc` is malformed, tampered with, or was
/// made under a different `SECRET_KEY`.
pub fn decrypt_link_token(secret_key: &[u8], enc: &str) -> Option<String> {
    let raw = URL_SAFE_NO_PAD.decode(enc).ok()?;
    if raw.len() <= NONCE_LEN {
        return None;
    }
    let (nonce, ciphertext) = raw.split_at(NONCE_LEN);
    let plain = cipher(secret_key)
        .decrypt(XNonce::from_slice(nonce), ciphertext)
        .ok()?;
    String::from_utf8(plain).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"test-secret-key-test-secret-key-0123";

    #[test]
    fn round_trips() {
        let enc = encrypt_link_token(KEY, "crsl_abc");
        assert_eq!(decrypt_link_token(KEY, &enc).as_deref(), Some("crsl_abc"));
    }

    #[test]
    fn nonce_is_random_and_token_not_visible() {
        let a = encrypt_link_token(KEY, "crsl_abc");
        let b = encrypt_link_token(KEY, "crsl_abc");
        assert_ne!(a, b);
        assert!(!a.contains("crsl_abc"));
    }

    #[test]
    fn tampering_is_rejected() {
        let enc = encrypt_link_token(KEY, "crsl_abc");
        let mut raw = URL_SAFE_NO_PAD.decode(&enc).unwrap();
        for i in [0, NONCE_LEN, raw.len() - 1] {
            raw[i] ^= 1;
            assert_eq!(decrypt_link_token(KEY, &URL_SAFE_NO_PAD.encode(&raw)), None);
            raw[i] ^= 1;
        }
    }

    #[test]
    fn wrong_key_and_garbage_are_rejected() {
        let enc = encrypt_link_token(KEY, "crsl_abc");
        assert_eq!(
            decrypt_link_token(b"another-key-another-key-another!", &enc),
            None
        );
        assert_eq!(decrypt_link_token(KEY, ""), None);
        assert_eq!(decrypt_link_token(KEY, "!!!"), None);
        assert_eq!(decrypt_link_token(KEY, "AAAA"), None);
    }
}
