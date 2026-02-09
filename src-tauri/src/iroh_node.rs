//! Iroh endpoint management for p2p networking
//!
//! This module manages the Iroh endpoint lifecycle, supporting both
//! on-demand (start for specific chat) and background modes.

use crate::iroh_derive::derive_iroh_keypair;
use iroh_base::key::NodeId;
#[allow(deprecated)]
use iroh_net::endpoint::Endpoint;
#[allow(deprecated)]
use iroh_net::relay::RelayMode;
use iroh_quinn::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// ALPN protocol identifier for SneakerNet chat
pub const CHAT_ALPN: &[u8] = b"sneakernet-chat/1";

#[derive(Error, Debug)]
pub enum IrohError {
    #[error("Iroh endpoint not started")]
    NotStarted,
    #[error("Iroh endpoint already running")]
    AlreadyRunning,
    #[error("Failed to create endpoint: {0}")]
    EndpointCreation(String),
    #[error("Failed to connect: {0}")]
    ConnectionFailed(String),
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
    #[error("Invalid node ID: {0}")]
    InvalidNodeId(String),
}

/// Iroh endpoint status
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IrohStatus {
    pub running: bool,
    pub node_id: Option<String>,
    pub relay_url: Option<String>,
    pub connected_contacts: Vec<String>,
}

/// Configuration for the Iroh node
#[derive(Clone, Debug)]
pub struct IrohConfig {
    /// Whether to use relay servers
    pub use_relays: bool,
    /// Custom relay URL (None = use default n0 relays)
    pub custom_relay_url: Option<String>,
}

impl Default for IrohConfig {
    fn default() -> Self {
        Self {
            use_relays: true,
            custom_relay_url: None,
        }
    }
}

/// Managed Iroh node state
pub struct IrohNode {
    endpoint: Option<Endpoint>,
    config: IrohConfig,
    /// Current contact we're connected with (their nostr pubkey)
    current_contact: Option<String>,
    /// Active connections keyed by contact pubkey
    connections: std::collections::HashMap<String, Connection>,
}

impl IrohNode {
    pub fn new(config: IrohConfig) -> Self {
        Self {
            endpoint: None,
            config,
            current_contact: None,
            connections: std::collections::HashMap::new(),
        }
    }

    /// Start the Iroh endpoint for a specific contact
    pub async fn start_for_contact(
        &mut self,
        nostr_secret_key: &[u8],
        my_pubkey_hex: &str,
        their_pubkey_hex: &str,
    ) -> Result<String, IrohError> {
        if self.endpoint.is_some() {
            return Err(IrohError::AlreadyRunning);
        }

        // Derive Iroh keypair for this contact relationship
        let (secret_key, _) = derive_iroh_keypair(nostr_secret_key, my_pubkey_hex, their_pubkey_hex)
            .map_err(|e| IrohError::KeyDerivation(e.to_string()))?;

        // Determine relay mode
        let relay_mode = if self.config.use_relays {
            RelayMode::Default
        } else {
            RelayMode::Disabled
        };

        // Create the endpoint
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .alpns(vec![CHAT_ALPN.to_vec()])
            .relay_mode(relay_mode)
            .bind()
            .await
            .map_err(|e| IrohError::EndpointCreation(e.to_string()))?;

        let node_id = endpoint.node_id().to_string();
        self.endpoint = Some(endpoint);
        self.current_contact = Some(their_pubkey_hex.to_string());

        Ok(node_id)
    }

    /// Stop the Iroh endpoint
    pub async fn stop(&mut self) -> Result<(), IrohError> {
        if let Some(endpoint) = self.endpoint.take() {
            // Close all connections
            self.connections.clear();
            
            // Close the endpoint with code 0 and empty reason
            let _ = endpoint.close(iroh_quinn::VarInt::from_u32(0), b"shutdown").await;
            
            self.current_contact = None;
        }
        Ok(())
    }

    /// Get current status
    pub fn status(&self) -> IrohStatus {
        IrohStatus {
            running: self.endpoint.is_some(),
            node_id: self.endpoint.as_ref().map(|e| e.node_id().to_string()),
            relay_url: None, // Could be populated from endpoint if needed
            connected_contacts: self.connections.keys().cloned().collect(),
        }
    }

    /// Connect to a contact's Iroh endpoint
    pub async fn connect_to_contact(
        &mut self,
        their_node_id: &str,
        contact_pubkey: &str,
    ) -> Result<(), IrohError> {
        let endpoint = self.endpoint.as_ref().ok_or(IrohError::NotStarted)?;

        // Parse their node ID (it's a public key in base32)
        let node_id: NodeId = their_node_id
            .parse()
            .map_err(|e: iroh_base::key::KeyParsingError| IrohError::InvalidNodeId(e.to_string()))?;

        // Connect using just the node ID - Iroh will use relays if needed
        let conn = endpoint
            .connect(node_id, CHAT_ALPN)
            .await
            .map_err(|e| IrohError::ConnectionFailed(e.to_string()))?;

        self.connections.insert(contact_pubkey.to_string(), conn);

        Ok(())
    }

    /// Get a connection for a contact
    pub fn get_connection(&self, contact_pubkey: &str) -> Option<&Connection> {
        self.connections.get(contact_pubkey)
    }

    /// Get mutable connection for a contact
    pub fn get_connection_mut(&mut self, contact_pubkey: &str) -> Option<&mut Connection> {
        self.connections.get_mut(contact_pubkey)
    }

    /// Get the endpoint reference
    pub fn endpoint(&self) -> Option<&Endpoint> {
        self.endpoint.as_ref()
    }
}

/// Thread-safe wrapper for IrohNode
pub type SharedIrohNode = Arc<RwLock<IrohNode>>;

/// Create a new shared Iroh node
pub fn create_shared_node(config: IrohConfig) -> SharedIrohNode {
    Arc::new(RwLock::new(IrohNode::new(config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = IrohConfig::default();
        assert!(config.use_relays);
        assert!(config.custom_relay_url.is_none());
    }

    #[test]
    fn test_status_not_running() {
        let node = IrohNode::new(IrohConfig::default());
        let status = node.status();
        assert!(!status.running);
        assert!(status.node_id.is_none());
    }
}
