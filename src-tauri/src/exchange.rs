//! NFC exchange protocol - message format, signing, and verification

use nostr::prelude::*;
use nostr::secp256k1::{self, Message as Secp256k1Message, Secp256k1, XOnlyPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// MIME type for NDEF records
pub const NDEF_MIME_TYPE: &str = "application/x-sneakernet";

#[derive(Error, Debug)]
pub enum ExchangeError {
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Protocol version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: u32, got: u32 },
    #[error("Invalid pubkey in message")]
    InvalidPubkey,
    #[error("Their pubkey doesn't match expected")]
    PubkeyMismatch,
    #[error("Message too old (timestamp check failed)")]
    MessageExpired,
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Signing error: {0}")]
    SigningError(String),
}

/// Exchange message sent over NFC
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeMessage {
    pub version: u32,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub pubkey: String,               // Our pubkey (hex)
    pub their_pubkey: Option<String>, // Their pubkey if known (hex)
    pub timestamp: u64,
    pub nonce: String,     // Random nonce (hex)
    pub signature: String, // Schnorr signature (hex)
}

/// Contact stored after successful exchange
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: String,
    pub nostr_pubkey: String,     // Their Nostr pubkey (hex)
    pub iroh_endpoint_id: String, // Derived Iroh endpoint ID
    pub exchanged_at: u64,        // Unix timestamp
    pub nickname: Option<String>,
}

/// Hash content for signing using SHA256
fn hash_content(content: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

impl ExchangeMessage {
    /// Create a new exchange message (initial broadcast, no their_pubkey yet)
    pub fn new_initial(keys: &Keys) -> Result<Self, ExchangeError> {
        Self::new(keys, None)
    }

    /// Create a new exchange message (response, includes their_pubkey)
    pub fn new_response(keys: &Keys, their_pubkey: &str) -> Result<Self, ExchangeError> {
        Self::new(keys, Some(their_pubkey.to_string()))
    }

    fn new(keys: &Keys, their_pubkey: Option<String>) -> Result<Self, ExchangeError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Generate random nonce
        let mut nonce_bytes = [0u8; 16];
        getrandom::getrandom(&mut nonce_bytes)
            .map_err(|e| ExchangeError::SerializationError(e.to_string()))?;
        let nonce = hex::encode(nonce_bytes);

        let pubkey = keys.public_key().to_hex();

        // Create the content to sign
        let content = format!(
            "sneakernet:{}:{}:{}:{}",
            pubkey,
            their_pubkey.as_deref().unwrap_or(""),
            timestamp,
            nonce
        );

        // Hash the content to create a message for signing
        let hash = hash_content(content.as_bytes());
        let message = Secp256k1Message::from_digest(hash);

        // Sign the message using the secret key
        let secp = Secp256k1::new();
        let secret_key = keys.secret_key();

        // Get the raw secp256k1 keypair
        let sk_bytes = hex::decode(secret_key.to_secret_hex())
            .map_err(|e| ExchangeError::SigningError(e.to_string()))?;
        let sk = secp256k1::SecretKey::from_slice(&sk_bytes)
            .map_err(|e| ExchangeError::SigningError(e.to_string()))?;
        let keypair = secp256k1::Keypair::from_secret_key(&secp, &sk);

        let signature = secp.sign_schnorr(&message, &keypair);

        Ok(Self {
            version: PROTOCOL_VERSION,
            msg_type: "sneakernet-exchange".to_string(),
            pubkey,
            their_pubkey,
            timestamp,
            nonce,
            signature: hex::encode(signature.serialize()),
        })
    }

    /// Serialize to JSON for NFC transmission
    pub fn to_json(&self) -> Result<String, ExchangeError> {
        serde_json::to_string(self).map_err(|e| ExchangeError::SerializationError(e.to_string()))
    }

    /// Deserialize from JSON received via NFC
    pub fn from_json(json: &str) -> Result<Self, ExchangeError> {
        serde_json::from_str(json).map_err(|e| ExchangeError::InvalidFormat(e.to_string()))
    }

    /// Verify the message signature and optionally check their_pubkey
    pub fn verify(&self, expected_our_pubkey: Option<&str>) -> Result<(), ExchangeError> {
        // Check version
        if self.version != PROTOCOL_VERSION {
            return Err(ExchangeError::VersionMismatch {
                expected: PROTOCOL_VERSION,
                got: self.version,
            });
        }

        // Check message type
        if self.msg_type != "sneakernet-exchange" {
            return Err(ExchangeError::InvalidFormat(
                "Invalid message type".to_string(),
            ));
        }

        // Parse the sender's public key
        let sender_pubkey =
            PublicKey::from_hex(&self.pubkey).map_err(|_| ExchangeError::InvalidPubkey)?;

        // Reconstruct the signed content
        let content = format!(
            "sneakernet:{}:{}:{}:{}",
            self.pubkey,
            self.their_pubkey.as_deref().unwrap_or(""),
            self.timestamp,
            self.nonce
        );

        // Hash the content
        let hash = hash_content(content.as_bytes());
        let message = Secp256k1Message::from_digest(hash);

        // Parse signature from hex
        let sig_bytes =
            hex::decode(&self.signature).map_err(|_| ExchangeError::SignatureVerificationFailed)?;
        let signature = secp256k1::schnorr::Signature::from_slice(&sig_bytes)
            .map_err(|_| ExchangeError::SignatureVerificationFailed)?;

        // Get the x-only pubkey for verification
        let xonly_pubkey = sender_pubkey.to_bytes();
        let xonly =
            XOnlyPublicKey::from_slice(&xonly_pubkey).map_err(|_| ExchangeError::InvalidPubkey)?;

        // Verify signature
        let secp = Secp256k1::verification_only();
        secp.verify_schnorr(&signature, &message, &xonly)
            .map_err(|_| ExchangeError::SignatureVerificationFailed)?;

        // If we expect our pubkey to be in their message, verify it
        if let Some(our_pubkey) = expected_our_pubkey {
            if let Some(ref their_claim) = self.their_pubkey {
                if their_claim != our_pubkey {
                    return Err(ExchangeError::PubkeyMismatch);
                }
            }
        }

        // Optional: Check timestamp isn't too old (e.g., 5 minutes)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now > self.timestamp && now - self.timestamp > 300 {
            return Err(ExchangeError::MessageExpired);
        }

        Ok(())
    }
}

impl Contact {
    /// Create a new contact from a verified exchange
    pub fn new(their_pubkey: &str, iroh_endpoint_id: &str) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: Uuid::new_v4().to_string(),
            nostr_pubkey: their_pubkey.to_string(),
            iroh_endpoint_id: iroh_endpoint_id.to_string(),
            exchanged_at: timestamp,
            nickname: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_initial_message() {
        let keys = Keys::generate();
        let msg = ExchangeMessage::new_initial(&keys).unwrap();

        assert_eq!(msg.version, PROTOCOL_VERSION);
        assert_eq!(msg.msg_type, "sneakernet-exchange");
        assert_eq!(msg.pubkey, keys.public_key().to_hex());
        assert!(msg.their_pubkey.is_none());
        assert!(!msg.nonce.is_empty());
        assert!(!msg.signature.is_empty());
    }

    #[test]
    fn test_create_response_message() {
        let keys = Keys::generate();
        let other_keys = Keys::generate();
        let their_pubkey = other_keys.public_key().to_hex();

        let msg = ExchangeMessage::new_response(&keys, &their_pubkey).unwrap();

        assert_eq!(msg.their_pubkey, Some(their_pubkey));
    }

    #[test]
    fn test_verify_message() {
        let keys = Keys::generate();
        let msg = ExchangeMessage::new_initial(&keys).unwrap();

        // Should verify successfully
        msg.verify(None).unwrap();
    }

    #[test]
    fn test_verify_response_with_our_pubkey() {
        let our_keys = Keys::generate();
        let their_keys = Keys::generate();
        let our_pubkey = our_keys.public_key().to_hex();

        // They create a response that includes our pubkey
        let msg = ExchangeMessage::new_response(&their_keys, &our_pubkey).unwrap();

        // Verify it includes our pubkey correctly
        msg.verify(Some(&our_pubkey)).unwrap();
    }

    #[test]
    fn test_verify_fails_on_wrong_pubkey() {
        let their_keys = Keys::generate();
        let wrong_keys = Keys::generate();

        // They create a response with wrong pubkey
        let msg =
            ExchangeMessage::new_response(&their_keys, &wrong_keys.public_key().to_hex()).unwrap();

        // Verify with different expected pubkey should fail
        let our_pubkey = Keys::generate().public_key().to_hex();
        let result = msg.verify(Some(&our_pubkey));

        assert!(matches!(result, Err(ExchangeError::PubkeyMismatch)));
    }

    #[test]
    fn test_json_roundtrip() {
        let keys = Keys::generate();
        let msg = ExchangeMessage::new_initial(&keys).unwrap();

        let json = msg.to_json().unwrap();
        let restored = ExchangeMessage::from_json(&json).unwrap();

        assert_eq!(msg.pubkey, restored.pubkey);
        assert_eq!(msg.signature, restored.signature);
    }

    #[test]
    fn test_contact_creation() {
        let contact = Contact::new("abcd1234", "endpoint-id-here");

        assert!(!contact.id.is_empty());
        assert_eq!(contact.nostr_pubkey, "abcd1234");
        assert!(contact.exchanged_at > 0);
    }
}
