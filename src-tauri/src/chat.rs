//! Chat protocol implementation over Iroh
//!
//! Simple text messaging between contacts using Iroh's QUIC streams.

use iroh_quinn::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
#[allow(unused_imports)]
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Maximum message size (64KB)
const MAX_MESSAGE_SIZE: usize = 65536;

#[derive(Error, Debug)]
pub enum ChatError {
    #[error("Not connected to contact")]
    NotConnected,
    #[error("Failed to send message: {0}")]
    SendFailed(String),
    #[error("Failed to receive message: {0}")]
    ReceiveFailed(String),
    #[error("Message too large")]
    MessageTooLarge,
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
}

/// A chat message
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub content: String,
    pub sender_pubkey: String,
    pub timestamp: u64,
    pub is_outgoing: bool,
}

impl ChatMessage {
    /// Create a new outgoing message
    pub fn new_outgoing(content: &str, sender_pubkey: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: content.to_string(),
            sender_pubkey: sender_pubkey.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            is_outgoing: true,
        }
    }

    /// Create from received wire format
    fn from_wire(data: &[u8], sender_pubkey: &str) -> Result<Self, ChatError> {
        let wire: WireMessage =
            serde_json::from_slice(data).map_err(|e| ChatError::InvalidFormat(e.to_string()))?;

        Ok(Self {
            id: wire.id,
            content: wire.content,
            sender_pubkey: sender_pubkey.to_string(),
            timestamp: wire.timestamp,
            is_outgoing: false,
        })
    }

    /// Convert to wire format
    fn to_wire(&self) -> Result<Vec<u8>, ChatError> {
        let wire = WireMessage {
            id: self.id.clone(),
            content: self.content.clone(),
            timestamp: self.timestamp,
        };

        serde_json::to_vec(&wire).map_err(|e| ChatError::SendFailed(e.to_string()))
    }
}

/// Wire format for messages (minimal, without local-only fields)
#[derive(Serialize, Deserialize)]
struct WireMessage {
    id: String,
    content: String,
    timestamp: u64,
}

/// Chat session with a contact
pub struct ChatSession {
    /// Contact's Nostr pubkey
    #[allow(dead_code)]
    contact_pubkey: String,
    /// Message history (in-memory, configurable persistence later)
    messages: Vec<ChatMessage>,
    /// Whether to persist messages
    #[allow(dead_code)]
    persist: bool,
}

impl ChatSession {
    pub fn new(contact_pubkey: &str, persist: bool) -> Self {
        Self {
            contact_pubkey: contact_pubkey.to_string(),
            messages: Vec::new(),
            persist,
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Get all messages
    pub fn get_messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Clear messages (for session-only mode)
    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

/// Chat manager handling multiple chat sessions
pub struct ChatManager {
    /// Sessions keyed by contact pubkey
    sessions: HashMap<String, ChatSession>,
    /// Our pubkey for identifying outgoing messages
    our_pubkey: String,
    /// Default persistence setting
    default_persist: bool,
}

impl ChatManager {
    pub fn new(our_pubkey: &str, default_persist: bool) -> Self {
        Self {
            sessions: HashMap::new(),
            our_pubkey: our_pubkey.to_string(),
            default_persist,
        }
    }

    /// Get or create a session for a contact
    pub fn get_or_create_session(&mut self, contact_pubkey: &str) -> &mut ChatSession {
        self.sessions
            .entry(contact_pubkey.to_string())
            .or_insert_with(|| ChatSession::new(contact_pubkey, self.default_persist))
    }

    /// Get session if it exists
    pub fn get_session(&self, contact_pubkey: &str) -> Option<&ChatSession> {
        self.sessions.get(contact_pubkey)
    }

    /// Send a message to a contact over an Iroh connection
    pub async fn send_message(
        &mut self,
        connection: &Connection,
        contact_pubkey: &str,
        content: &str,
    ) -> Result<ChatMessage, ChatError> {
        // Create the message
        let message = ChatMessage::new_outgoing(content, &self.our_pubkey);

        // Serialize to wire format
        let data = message.to_wire()?;

        if data.len() > MAX_MESSAGE_SIZE {
            return Err(ChatError::MessageTooLarge);
        }

        // Open a unidirectional stream and send
        let mut send_stream = connection
            .open_uni()
            .await
            .map_err(|e| ChatError::SendFailed(e.to_string()))?;

        // Write length prefix (4 bytes, big endian)
        let len_bytes = (data.len() as u32).to_be_bytes();
        send_stream
            .write_all(&len_bytes)
            .await
            .map_err(|e| ChatError::SendFailed(e.to_string()))?;

        // Write the message
        send_stream
            .write_all(&data)
            .await
            .map_err(|e| ChatError::SendFailed(e.to_string()))?;

        // Finish the stream
        send_stream
            .finish()
            .map_err(|e| ChatError::SendFailed(e.to_string()))?;

        // Add to session
        let session = self.get_or_create_session(contact_pubkey);
        session.add_message(message.clone());

        Ok(message)
    }

    /// Receive a message from a unidirectional stream
    pub async fn receive_message(
        &mut self,
        connection: &Connection,
        sender_pubkey: &str,
    ) -> Result<ChatMessage, ChatError> {
        // Accept a unidirectional stream
        let mut recv_stream = connection
            .accept_uni()
            .await
            .map_err(|e| ChatError::ReceiveFailed(e.to_string()))?;

        // Read length prefix
        let mut len_bytes = [0u8; 4];
        recv_stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(|e| ChatError::ReceiveFailed(e.to_string()))?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        if len > MAX_MESSAGE_SIZE {
            return Err(ChatError::MessageTooLarge);
        }

        // Read the message
        let mut data = vec![0u8; len];
        recv_stream
            .read_exact(&mut data)
            .await
            .map_err(|e| ChatError::ReceiveFailed(e.to_string()))?;

        // Parse the message
        let message = ChatMessage::from_wire(&data, sender_pubkey)?;

        // Add to session
        let session = self.get_or_create_session(sender_pubkey);
        session.add_message(message.clone());

        Ok(message)
    }

    /// Get messages for a contact
    pub fn get_messages(&self, contact_pubkey: &str) -> Vec<ChatMessage> {
        self.get_session(contact_pubkey)
            .map(|s| s.get_messages().to_vec())
            .unwrap_or_default()
    }

    /// Clear all sessions (for cleanup)
    pub fn clear_all(&mut self) {
        self.sessions.clear();
    }
}

/// Thread-safe wrapper for ChatManager
pub type SharedChatManager = Arc<RwLock<Option<ChatManager>>>;

/// Create a new shared chat manager
pub fn create_shared_manager() -> SharedChatManager {
    Arc::new(RwLock::new(None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_outgoing_message() {
        let msg = ChatMessage::new_outgoing("Hello!", "abc123");

        assert!(!msg.id.is_empty());
        assert_eq!(msg.content, "Hello!");
        assert_eq!(msg.sender_pubkey, "abc123");
        assert!(msg.is_outgoing);
        assert!(msg.timestamp > 0);
    }

    #[test]
    fn test_wire_roundtrip() {
        let msg = ChatMessage::new_outgoing("Test message", "sender");
        let wire = msg.to_wire().unwrap();
        let restored = ChatMessage::from_wire(&wire, "sender").unwrap();

        assert_eq!(msg.id, restored.id);
        assert_eq!(msg.content, restored.content);
        assert_eq!(msg.timestamp, restored.timestamp);
        // is_outgoing will be false since it's "received"
        assert!(!restored.is_outgoing);
    }

    #[test]
    fn test_chat_session() {
        let mut session = ChatSession::new("contact123", false);

        let msg = ChatMessage::new_outgoing("Hi", "me");
        session.add_message(msg);

        assert_eq!(session.get_messages().len(), 1);

        session.clear();
        assert!(session.get_messages().is_empty());
    }

    #[test]
    fn test_chat_manager() {
        let mut manager = ChatManager::new("my_pubkey", false);

        // Get or create session
        let session = manager.get_or_create_session("contact1");
        session.add_message(ChatMessage::new_outgoing("Test", "my_pubkey"));

        let messages = manager.get_messages("contact1");
        assert_eq!(messages.len(), 1);

        // Non-existent contact returns empty
        let messages = manager.get_messages("contact2");
        assert!(messages.is_empty());
    }
}
