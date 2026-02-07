//! Iroh key derivation from Nostr keys and exchange context

use hkdf::Hkdf;
use iroh_base::key::{PublicKey as IrohPublicKey, SecretKey as IrohSecretKey};
use sha2::Sha256;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeriveError {
    #[error("Invalid secret key length")]
    InvalidSecretKeyLength,
    #[error("Invalid public key format: {0}")]
    InvalidPublicKey(String),
    #[error("HKDF expansion failed")]
    HkdfExpansionFailed,
}

/// Derive an Iroh keypair from a Nostr secret key and exchange context.
///
/// The derivation uses HKDF-SHA256 with:
/// - IKM (Input Key Material): Nostr secret key bytes
/// - Salt: SHA256 hash of sorted pubkeys (ensures same result regardless of who initiates)
/// - Info: "sneakernet-iroh-v1" context string
///
/// This ensures:
/// 1. Deterministic: Same inputs always produce same Iroh key
/// 2. Unique per relationship: Different contact = different Iroh identity
/// 3. Secure: HKDF is a standard, secure key derivation function
pub fn derive_iroh_keypair(
    nostr_secret_key: &[u8],
    my_pubkey_hex: &str,
    their_pubkey_hex: &str,
) -> Result<(IrohSecretKey, IrohPublicKey), DeriveError> {
    // Validate input
    if nostr_secret_key.len() != 32 {
        return Err(DeriveError::InvalidSecretKeyLength);
    }

    // Decode pubkeys from hex
    let my_pubkey_bytes =
        hex::decode(my_pubkey_hex).map_err(|e| DeriveError::InvalidPublicKey(e.to_string()))?;
    let their_pubkey_bytes =
        hex::decode(their_pubkey_hex).map_err(|e| DeriveError::InvalidPublicKey(e.to_string()))?;

    // Sort pubkeys to ensure same salt regardless of who initiates
    let (first, second) = if my_pubkey_bytes < their_pubkey_bytes {
        (&my_pubkey_bytes, &their_pubkey_bytes)
    } else {
        (&their_pubkey_bytes, &my_pubkey_bytes)
    };

    // Create salt from sorted pubkeys
    use sha2::Digest;
    let mut hasher = Sha256::new();
    hasher.update(first);
    hasher.update(second);
    let salt = hasher.finalize();

    // HKDF-SHA256 key derivation
    let hk = Hkdf::<Sha256>::new(Some(&salt), nostr_secret_key);

    let mut iroh_seed = [0u8; 32];
    hk.expand(b"sneakernet-iroh-v1", &mut iroh_seed)
        .map_err(|_| DeriveError::HkdfExpansionFailed)?;

    // Create Iroh keypair from seed
    let secret_key = IrohSecretKey::from_bytes(&iroh_seed);
    let public_key = secret_key.public();

    Ok((secret_key, public_key))
}

/// Get the Iroh endpoint ID (public key in base32) from derived keys
pub fn get_endpoint_id(public_key: &IrohPublicKey) -> String {
    public_key.to_string()
}

/// Derive and return just the endpoint ID (convenience function)
pub fn derive_endpoint_id(
    nostr_secret_key: &[u8],
    my_pubkey_hex: &str,
    their_pubkey_hex: &str,
) -> Result<String, DeriveError> {
    let (_, public_key) = derive_iroh_keypair(nostr_secret_key, my_pubkey_hex, their_pubkey_hex)?;
    Ok(get_endpoint_id(&public_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_keypair() {
        let nostr_secret = [0x42u8; 32]; // Test secret key
        let my_pubkey = "a".repeat(64);
        let their_pubkey = "b".repeat(64);

        let result = derive_iroh_keypair(&nostr_secret, &my_pubkey, &their_pubkey);
        assert!(result.is_ok());

        let (secret, public) = result.unwrap();
        assert_eq!(secret.public(), public);
    }

    #[test]
    fn test_deterministic_derivation() {
        let nostr_secret = [0x42u8; 32];
        let my_pubkey = "a".repeat(64);
        let their_pubkey = "b".repeat(64);

        let (_, public1) = derive_iroh_keypair(&nostr_secret, &my_pubkey, &their_pubkey).unwrap();
        let (_, public2) = derive_iroh_keypair(&nostr_secret, &my_pubkey, &their_pubkey).unwrap();

        assert_eq!(public1, public2);
    }

    #[test]
    fn test_order_independent() {
        // The result should be the same regardless of who initiates
        let nostr_secret = [0x42u8; 32];
        let pubkey_a = "a".repeat(64);
        let pubkey_b = "b".repeat(64);

        let (_, public1) = derive_iroh_keypair(&nostr_secret, &pubkey_a, &pubkey_b).unwrap();
        let (_, public2) = derive_iroh_keypair(&nostr_secret, &pubkey_b, &pubkey_a).unwrap();

        assert_eq!(public1, public2);
    }

    #[test]
    fn test_different_contacts_different_keys() {
        let nostr_secret = [0x42u8; 32];
        let my_pubkey = "a".repeat(64);
        let contact1_pubkey = "b".repeat(64);
        let contact2_pubkey = "c".repeat(64);

        let (_, public1) =
            derive_iroh_keypair(&nostr_secret, &my_pubkey, &contact1_pubkey).unwrap();
        let (_, public2) =
            derive_iroh_keypair(&nostr_secret, &my_pubkey, &contact2_pubkey).unwrap();

        assert_ne!(public1, public2);
    }

    #[test]
    fn test_endpoint_id_format() {
        let nostr_secret = [0x42u8; 32];
        let my_pubkey = "a".repeat(64);
        let their_pubkey = "b".repeat(64);

        let endpoint_id = derive_endpoint_id(&nostr_secret, &my_pubkey, &their_pubkey).unwrap();

        // Iroh endpoint IDs are base32 encoded
        assert!(!endpoint_id.is_empty());
        // They should be alphanumeric (base32)
        assert!(endpoint_id.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_invalid_secret_key_length() {
        let short_secret = [0x42u8; 16]; // Too short
        let my_pubkey = "a".repeat(64);
        let their_pubkey = "b".repeat(64);

        let result = derive_iroh_keypair(&short_secret, &my_pubkey, &their_pubkey);
        assert!(matches!(result, Err(DeriveError::InvalidSecretKeyLength)));
    }

    #[test]
    fn test_invalid_pubkey_format() {
        let nostr_secret = [0x42u8; 32];
        let invalid_pubkey = "not-hex!";
        let their_pubkey = "b".repeat(64);

        let result = derive_iroh_keypair(&nostr_secret, invalid_pubkey, &their_pubkey);
        assert!(matches!(result, Err(DeriveError::InvalidPublicKey(_))));
    }
}
