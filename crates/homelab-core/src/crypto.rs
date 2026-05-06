use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, Nonce};

use crate::HomelabError;

#[derive(Clone)]
pub struct SecretsCipher {
    cipher: Aes256Gcm,
}

impl SecretsCipher {
    pub fn new(key_hex: &str) -> Result<Self, HomelabError> {
        let key_bytes = hex_decode(key_hex).map_err(|e| {
            HomelabError::Encryption(format!("invalid encryption key hex: {e}"))
        })?;
        if key_bytes.len() != 32 {
            return Err(HomelabError::Encryption(format!(
                "encryption key must be 32 bytes, got {}",
                key_bytes.len()
            )));
        }
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        Ok(Self { cipher })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<(Vec<u8>, Vec<u8>), HomelabError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| HomelabError::Encryption(format!("encrypt failed: {e}")))?;
        Ok((ciphertext, nonce.to_vec()))
    }

    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8]) -> Result<String, HomelabError> {
        let nonce = Nonce::from_slice(nonce);
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| HomelabError::Encryption(format!("decrypt failed: {e}")))?;
        String::from_utf8(plaintext)
            .map_err(|e| HomelabError::Encryption(format!("invalid utf8: {e}")))
    }
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) {
        return Err("odd-length hex string".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> String {
        "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string()
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let cipher = SecretsCipher::new(&test_key()).unwrap();
        let plaintext = "super-secret-value";
        let (ciphertext, nonce) = cipher.encrypt(plaintext).unwrap();
        let decrypted = cipher.decrypt(&ciphertext, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn different_nonce_per_encrypt() {
        let cipher = SecretsCipher::new(&test_key()).unwrap();
        let (_, nonce1) = cipher.encrypt("same").unwrap();
        let (_, nonce2) = cipher.encrypt("same").unwrap();
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn rejects_bad_key_length() {
        assert!(SecretsCipher::new("abcd").is_err());
    }

    #[test]
    fn rejects_invalid_hex() {
        assert!(SecretsCipher::new("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err());
    }
}
