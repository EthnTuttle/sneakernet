# SneakerNet Roadmap

## Overview

SneakerNet is an Android app built with Tauri v2 that enables secure cryptographic key exchange via NFC tap. It implements a Nostr-based key exchange protocol and derives Iroh p2p identity keys from the exchange.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Frontend (Web)                     │
│                 Solid.js + TypeScript                │
├─────────────────────────────────────────────────────┤
│                  Tauri Bridge                        │
├─────────────────────────────────────────────────────┤
│                 Rust Backend                         │
│  ┌───────────┐  ┌───────────┐  ┌──────────────────┐ │
│  │   Nostr   │  │   Iroh    │  │  NFC Handler     │ │
│  │   Keys    │  │Key Derive │  │  (Tauri Plugin)  │ │
│  └───────────┘  └───────────┘  └──────────────────┘ │
└─────────────────────────────────────────────────────┘
```

## Key Exchange Protocol

### Phase 1: Initial Broadcast
- Device A starts NFC scan mode
- Device B taps and sends its public key
- Device A receives B's pubkey

### Phase 2: Signed Response
- Device A creates a signed message containing:
  - A's public key
  - B's public key (proving knowledge of recipient)
  - Timestamp and nonce
- Device A writes this to NFC for B to read
- B verifies signature and that their pubkey is included

### Phase 3: Mutual Verification
- B creates and sends similar signed message back
- A verifies B's signature includes A's pubkey
- Exchange complete - both have verified each other's keys

### Phase 4: Iroh Key Derivation
- Each device derives an Iroh keypair using:
  - Their Nostr secret key as input key material
  - Sorted hash of both pubkeys as salt
  - "sneakernet-iroh-v1" as context info
- This creates deterministic, unique Iroh identities per relationship

## Project Structure

```
sneakernet/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs              # Tauri mobile entry, plugin init
│   │   ├── keys.rs             # Nostr key generation/management
│   │   ├── exchange.rs         # NFC exchange protocol logic
│   │   ├── iroh_derive.rs      # Iroh key derivation from Nostr
│   │   └── commands.rs         # Tauri command handlers
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json        # NFC + Store permissions
│   └── gen/                    # Generated Android project
├── src/
│   ├── index.html
│   ├── index.tsx               # Solid.js entry
│   ├── App.tsx                 # Main app component
│   ├── components/
│   │   ├── KeyDisplay.tsx      # Shows current keys
│   │   ├── NFCExchange.tsx     # NFC tap UI/status
│   │   └── ContactList.tsx     # Exchanged contacts
│   ├── lib/
│   │   ├── tauri.ts            # Tauri command bindings
│   │   └── types.ts            # TypeScript types
│   └── styles/
│       └── main.css
├── package.json
├── vite.config.ts
├── ROADMAP.md
└── CLAUDE.md
```

## Implementation Phases

### Phase 1: Project Setup
- [x] Initialize Tauri v2 project
- [x] Configure for Android target
- [x] Add Solid.js frontend with Vite
- [x] Add Tauri plugins: nfc, store
- [x] Configure Android permissions and capabilities

### Phase 2: Nostr Key Management
- [x] Implement `keys.rs` module
  - [x] Key generation using `nostr` crate
  - [x] Key serialization/deserialization
  - [x] Secure storage via Tauri Store
- [x] Tauri commands:
  - [x] `generate_keys` - Create new Nostr keypair
  - [x] `get_public_key` - Get current pubkey (hex and bech32)
  - [x] `has_keys` - Check if keys exist

### Phase 3: NFC Exchange Protocol
- [x] Implement `exchange.rs` module
  - [x] Define `ExchangeMessage` struct
  - [x] Define `SignedExchange` struct
  - [x] Implement message signing with Schnorr
  - [x] Implement signature verification
  - [x] NDEF record creation (MIME: `application/x-sneakernet`)
  - [x] NDEF record parsing
- [x] Implement polling exchange state machine
- [x] Tauri commands:
  - [x] `start_exchange` - Begin NFC scan
  - [x] `complete_exchange` - Finish and verify exchange

### Phase 4: Iroh Key Derivation
- [x] Implement `iroh_derive.rs` module
  - [x] HKDF-SHA256 key derivation
  - [x] Deterministic Iroh SecretKey generation
  - [x] EndpointId extraction
- [x] Store derived keys with contact association

### Phase 5: Frontend UI
- [x] Home screen
  - [x] Display own Nostr pubkey (npub format)
  - [x] "Start Exchange" button
  - [x] Key generation on first launch
- [x] Exchange screen
  - [x] NFC tap animation/indicator
  - [x] Status updates
  - [x] Success/failure feedback
- [x] Contacts screen
  - [x] List of exchanged contacts
  - [x] Show Nostr pubkey and Iroh EndpointId

## Data Structures

### Exchange Message (JSON over NDEF)
```json
{
  "version": 1,
  "type": "sneakernet-exchange",
  "pubkey": "hex-pubkey",
  "their_pubkey": "hex-pubkey",
  "timestamp": 1707235200,
  "nonce": "hex-random-16-bytes",
  "signature": "hex-schnorr-signature"
}
```

### Stored Contact
```json
{
  "id": "uuid",
  "nostr_pubkey": "hex-pubkey",
  "iroh_endpoint_id": "base32-endpoint-id",
  "exchanged_at": 1707235200,
  "nickname": null
}
```

## Future Enhancements (Post-PoC)

1. **QR Code Fallback** - For devices without NFC
2. **Iroh Connection Test** - Verify p2p connectivity after exchange
3. **Key Rotation** - Support updating keys while maintaining contacts
4. **Export/Import** - Backup and restore contacts
5. **NIP-05 Verification** - Verify Nostr identities
6. **Multi-device Sync** - Sync contacts across devices
