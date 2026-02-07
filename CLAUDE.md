# CLAUDE.md - AI Assistant Context

## Project Overview

**SneakerNet** is a Tauri v2 Android app for NFC-based cryptographic key exchange. It uses Nostr keys for identity and derives Iroh keys for p2p networking.

## Tech Stack

- **Frontend**: Solid.js + TypeScript + Vite
- **Backend**: Rust with Tauri v2
- **Mobile**: Android via Tauri mobile
- **Crypto**: nostr crate (secp256k1/Schnorr), HKDF-SHA256
- **P2P**: Iroh (ed25519 keys)
- **Storage**: Tauri Store plugin
- **NFC**: Tauri NFC plugin

## Key Commands

```bash
# Development
npm run tauri dev              # Desktop dev (limited, no NFC)
npm run tauri android dev      # Android dev with hot reload

# Building
npm run tauri android build    # Build Android APK/AAB

# Testing
cargo test --manifest-path src-tauri/Cargo.toml
```

## Architecture

### Rust Modules (`src-tauri/src/`)

- **lib.rs**: Tauri entry point, plugin initialization
- **keys.rs**: Nostr keypair generation and secure storage
- **exchange.rs**: NFC exchange protocol (message format, signing, verification)
- **iroh_derive.rs**: Derive Iroh keys from Nostr keys + exchange context
- **commands.rs**: Tauri command handlers exposed to frontend

### Frontend Components (`src/`)

- **App.tsx**: Main app with tab navigation
- **components/KeyDisplay.tsx**: Shows own Nostr pubkey
- **components/NFCExchange.tsx**: NFC exchange UI and state
- **components/ContactList.tsx**: List of exchanged contacts

### Key Data Flow

```
1. User taps "Exchange"
2. NFC scan starts → receives other device's pubkey
3. Creates signed message including their pubkey
4. Writes response via NFC
5. Verifies their signed response includes our pubkey
6. Derives Iroh key: HKDF(nostr_secret, sorted_pubkeys_hash, "sneakernet-iroh-v1")
7. Stores contact with Nostr pubkey + Iroh EndpointId
```

## NFC Exchange Protocol

### Message Format
```json
{
  "version": 1,
  "type": "sneakernet-exchange",
  "pubkey": "hex-nostr-pubkey",
  "their_pubkey": "hex-their-pubkey-or-null",
  "timestamp": 1707235200,
  "nonce": "hex-16-bytes",
  "signature": "hex-schnorr-sig"
}
```

### NDEF Configuration
- MIME type: `application/x-sneakernet`
- Payload: JSON (above format)

## Important Implementation Notes

1. **Nostr Keys**: Using `nostr` crate v0.44 with Schnorr signatures (BIP-340)
2. **Iroh Keys**: Using `iroh-base` for SecretKey/PublicKey (ed25519)
3. **Key Derivation**: HKDF-SHA256 ensures deterministic Iroh keys from Nostr identity
4. **NFC Polling**: Since Android Beam is deprecated, we use read/write polling
5. **Mobile-only Features**: NFC plugin only works on Android/iOS, not desktop

## Common Tasks

### Adding a new Tauri command:
1. Add function in `commands.rs`
2. Register in `lib.rs` invoke_handler
3. Add TypeScript binding in `src/lib/tauri.ts`

### Modifying exchange protocol:
1. Update `ExchangeMessage` struct in `exchange.rs`
2. Update signing/verification logic
3. Update frontend `NFCExchange.tsx` state machine

### Adding new contact fields:
1. Update `Contact` struct in `exchange.rs`
2. Update storage logic in `commands.rs`
3. Update `ContactList.tsx` display

## Testing

### Unit Tests
- `keys.rs`: Key generation, serialization
- `exchange.rs`: Message creation, signature verification
- `iroh_derive.rs`: Key derivation determinism

### Manual Testing
1. Build APK: `npm run tauri android build`
2. Install on two Android devices with NFC
3. Generate keys on both devices
4. Tap devices together and verify exchange completes
5. Check contacts list shows correct Iroh EndpointIds

## Troubleshooting

### NFC not working
- Check Android NFC is enabled in settings
- Ensure app has NFC permission granted
- Try holding devices together longer (2-3 seconds)

### Keys not persisting
- Check Tauri Store plugin is initialized
- Verify capabilities/default.json has store permissions

### Build failures
- Run `cargo check --manifest-path src-tauri/Cargo.toml` for Rust errors
- Check Android SDK is properly configured
- Ensure ANDROID_HOME and JAVA_HOME are set

## File Locations

| Purpose | Location |
|---------|----------|
| Tauri config | `src-tauri/tauri.conf.json` |
| Rust deps | `src-tauri/Cargo.toml` |
| Capabilities | `src-tauri/capabilities/default.json` |
| Android manifest | `src-tauri/gen/android/app/src/main/AndroidManifest.xml` |
| Frontend entry | `src/index.tsx` |
| Styles | `src/styles/main.css` |
