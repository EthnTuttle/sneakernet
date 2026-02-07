//! Nostr key generation and management

use nostr::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeyError {
    #[error("Failed to generate keys: {0}")]
    GenerationError(String),
    #[error("Failed to parse key: {0}")]
    ParseError(String),
    #[error("No keys found")]
    NoKeysFound,
}

/// Serializable key data for storage
#[derive(Serialize, Deserialize, Clone)]
pub struct StoredKeys {
    /// Secret key in hex format
    pub secret_key_hex: String,
    /// Public key in hex format  
    pub public_key_hex: String,
}

/// Public key info returned to frontend
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NostrKeysInfo {
    pub public_key: String,        // hex
    pub public_key_bech32: String, // npub
}

/// Generate a new Nostr keypair
pub fn generate_keypair() -> Result<(Keys, StoredKeys), KeyError> {
    let keys = Keys::generate();

    let stored = StoredKeys {
        secret_key_hex: keys.secret_key().to_secret_hex(),
        public_key_hex: keys.public_key().to_hex(),
    };

    Ok((keys, stored))
}

/// Restore keys from stored data
pub fn restore_keys(stored: &StoredKeys) -> Result<Keys, KeyError> {
    let secret_key = SecretKey::from_hex(&stored.secret_key_hex)
        .map_err(|e| KeyError::ParseError(e.to_string()))?;

    Ok(Keys::new(secret_key))
}

/// Get public key info from keys
pub fn get_public_key_info(keys: &Keys) -> Result<NostrKeysInfo, KeyError> {
    let public_key = keys.public_key();

    Ok(NostrKeysInfo {
        public_key: public_key.to_hex(),
        public_key_bech32: public_key
            .to_bech32()
            .map_err(|e| KeyError::ParseError(e.to_string()))?,
    })
}

/// Get public key info from stored keys
pub fn get_public_key_info_from_stored(stored: &StoredKeys) -> Result<NostrKeysInfo, KeyError> {
    let keys = restore_keys(stored)?;
    get_public_key_info(&keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let (keys, stored) = generate_keypair().unwrap();
        assert!(!stored.secret_key_hex.is_empty());
        assert!(!stored.public_key_hex.is_empty());
        assert_eq!(stored.public_key_hex, keys.public_key().to_hex());
    }

    #[test]
    fn test_key_restoration() {
        let (original_keys, stored) = generate_keypair().unwrap();
        let restored_keys = restore_keys(&stored).unwrap();

        assert_eq!(
            original_keys.public_key().to_hex(),
            restored_keys.public_key().to_hex()
        );
    }

    #[test]
    fn test_public_key_info() {
        let (keys, _) = generate_keypair().unwrap();
        let info = get_public_key_info(&keys).unwrap();

        assert!(info.public_key_bech32.starts_with("npub"));
        assert_eq!(info.public_key.len(), 64); // 32 bytes hex
    }
}
