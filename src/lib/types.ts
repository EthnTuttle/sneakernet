// Nostr key types
export interface NostrKeys {
  publicKey: string;      // hex-encoded public key
  publicKeyBech32: string; // npub format
}

// Contact from NFC exchange
export interface Contact {
  id: string;
  nostrPubkey: string;       // hex-encoded Nostr pubkey
  irohEndpointId: string;    // Iroh endpoint ID (base32)
  exchangedAt: number;       // Unix timestamp
  nickname: string | null;
}

// NFC Exchange message format
export interface ExchangeMessage {
  version: number;
  type: string;
  pubkey: string;
  theirPubkey: string | null;
  timestamp: number;
  nonce: string;
  signature: string;
}

// Exchange states
export type ExchangeStatus = 
  | { state: 'idle' }
  | { state: 'scanning' }
  | { state: 'received'; theirPubkey: string }
  | { state: 'sending' }
  | { state: 'verifying' }
  | { state: 'complete'; contact: Contact }
  | { state: 'error'; message: string };

// Exchange mode (NFC or QR)
export type ExchangeMode = 'nfc' | 'qr';

// QR Exchange states
export type QRExchangeStatus =
  | { state: 'idle' }
  | { state: 'showing-qr' }
  | { state: 'scanning' }
  | { state: 'processing'; theirPubkey: string }
  | { state: 'complete'; contact: Contact }
  | { state: 'error'; message: string };

// Tab navigation
export type TabId = 'keys' | 'exchange' | 'contacts' | 'chat';

// Iroh status
export interface IrohStatus {
  running: boolean;
  nodeId: string | null;
  relayUrl: string | null;
  connectedContacts: string[];
}

// Chat message
export interface ChatMessage {
  id: string;
  content: string;
  senderPubkey: string;
  timestamp: number;
  isOutgoing: boolean;
}
