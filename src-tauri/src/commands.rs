//! Tauri command handlers

use crate::exchange::{Contact, ExchangeMessage};
use crate::iroh_derive::derive_endpoint_id;
use crate::keys::{
    generate_keypair, get_public_key_info_from_stored, restore_keys, NostrKeysInfo, StoredKeys,
};
use serde_json::json;
use std::sync::Mutex;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

/// Application state
pub struct AppState {
    /// Cached keys (loaded from store on startup)
    pub keys: Mutex<Option<StoredKeys>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            keys: Mutex::new(None),
        }
    }
}

const STORE_FILE: &str = "sneakernet.json";
const KEYS_KEY: &str = "nostr_keys";
const CONTACTS_KEY: &str = "contacts";

/// Helper to load keys from store
fn load_keys_from_store(app: &AppHandle) -> Option<StoredKeys> {
    let store = app.store(STORE_FILE).ok()?;
    let value = store.get(KEYS_KEY)?;
    serde_json::from_value(value).ok()
}

/// Helper to save keys to store
fn save_keys_to_store(app: &AppHandle, keys: &StoredKeys) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(KEYS_KEY, json!(keys));
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

/// Helper to load contacts from store
fn load_contacts_from_store(app: &AppHandle) -> Vec<Contact> {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    
    match store.get(CONTACTS_KEY) {
        Some(value) => serde_json::from_value(value).unwrap_or_default(),
        None => vec![],
    }
}

/// Helper to save contacts to store
fn save_contacts_to_store(app: &AppHandle, contacts: &[Contact]) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    store.set(CONTACTS_KEY, json!(contacts));
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================================
// Key Management Commands
// ============================================================================

#[tauri::command]
pub fn has_keys(state: State<AppState>, app: AppHandle) -> bool {
    // First check cached state
    {
        let keys = state.keys.lock().unwrap();
        if keys.is_some() {
            return true;
        }
    }
    
    // Try to load from store
    if let Some(stored) = load_keys_from_store(&app) {
        let mut keys = state.keys.lock().unwrap();
        *keys = Some(stored);
        return true;
    }
    
    false
}

#[tauri::command]
pub fn generate_keys(state: State<AppState>, app: AppHandle) -> Result<NostrKeysInfo, String> {
    let (_, stored) = generate_keypair().map_err(|e| e.to_string())?;
    
    // Save to store
    save_keys_to_store(&app, &stored)?;
    
    // Cache in state
    {
        let mut keys = state.keys.lock().unwrap();
        *keys = Some(stored.clone());
    }
    
    get_public_key_info_from_stored(&stored).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_public_key(state: State<AppState>, app: AppHandle) -> Result<NostrKeysInfo, String> {
    // Check cache first
    {
        let keys = state.keys.lock().unwrap();
        if let Some(ref stored) = *keys {
            return get_public_key_info_from_stored(stored).map_err(|e| e.to_string());
        }
    }
    
    // Try to load from store
    let stored = load_keys_from_store(&app).ok_or("No keys found")?;
    
    // Cache it
    {
        let mut keys = state.keys.lock().unwrap();
        *keys = Some(stored.clone());
    }
    
    get_public_key_info_from_stored(&stored).map_err(|e| e.to_string())
}

// ============================================================================
// NFC Exchange Commands
// ============================================================================

#[tauri::command]
pub async fn is_nfc_available(app: AppHandle) -> Result<bool, String> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        use tauri_plugin_nfc::NfcExt;
        app.nfc()
            .is_available()
            .map_err(|e| e.to_string())
    }
    
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = app;
        Ok(false)
    }
}

#[tauri::command]
pub async fn start_nfc_scan(app: AppHandle) -> Result<String, String> {
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        use tauri_plugin_nfc::NfcExt;
        
        // Scan for NDEF tag
        let scan_result = app
            .nfc()
            .scan(tauri_plugin_nfc::ScanRequest {
                kind: tauri_plugin_nfc::ScanKind::Ndef {
                    mime_type: Some(crate::exchange::NDEF_MIME_TYPE.to_string()),
                    uri: None,
                    tech_list: None,
                },
                keep_session_alive: true,
            })
            .map_err(|e| e.to_string())?;
        
        // Extract the records from the tag
        let tag = scan_result.tag;
        
        // Find our record
        for record in tag.records {
            let payload_str = String::from_utf8(record.payload)
                .map_err(|e| e.to_string())?;
            
            // Try to parse the exchange message
            if let Ok(msg) = ExchangeMessage::from_json(&payload_str) {
                // Verify the message (basic verification, not checking their_pubkey yet)
                msg.verify(None).map_err(|e| e.to_string())?;
                
                return Ok(msg.pubkey);
            }
        }
        
        Err("No valid exchange message found".to_string())
    }
    
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = app;
        Err("NFC not supported on this platform".to_string())
    }
}

#[tauri::command]
pub async fn write_nfc_response(
    their_pubkey: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // Get our keys
    let stored = {
        let keys = state.keys.lock().unwrap();
        keys.clone().ok_or("No keys found")?
    };
    
    let our_keys = restore_keys(&stored).map_err(|e| e.to_string())?;
    
    // Create signed response that includes their pubkey
    let msg = ExchangeMessage::new_response(&our_keys, &their_pubkey)
        .map_err(|e| e.to_string())?;
    
    let json = msg.to_json().map_err(|e| e.to_string())?;
    
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        use tauri_plugin_nfc::{NfcRecord, NfcExt, NFCTypeNameFormat};
        
        // Write to NFC using Media type for MIME
        app.nfc()
            .write(vec![NfcRecord {
                format: NFCTypeNameFormat::Media,
                kind: crate::exchange::NDEF_MIME_TYPE.as_bytes().to_vec(),
                id: vec![],
                payload: json.into_bytes(),
            }])
            .map_err(|e| e.to_string())?;
        
        Ok(())
    }
    
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let _ = app;
        let _ = json;
        Err("NFC not supported on this platform".to_string())
    }
}

#[tauri::command]
pub async fn complete_exchange(
    their_pubkey: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Contact, String> {
    // Get our keys
    let stored = {
        let keys = state.keys.lock().unwrap();
        keys.clone().ok_or("No keys found")?
    };
    
    // Derive Iroh endpoint ID
    let secret_key_bytes = hex::decode(&stored.secret_key_hex)
        .map_err(|e| e.to_string())?;
    
    let iroh_endpoint_id = derive_endpoint_id(
        &secret_key_bytes,
        &stored.public_key_hex,
        &their_pubkey,
    )
    .map_err(|e| e.to_string())?;
    
    // Create contact
    let contact = Contact::new(&their_pubkey, &iroh_endpoint_id);
    
    // Load existing contacts, add new one, save
    let mut contacts = load_contacts_from_store(&app);
    
    // Check if contact already exists (by pubkey)
    if !contacts.iter().any(|c| c.nostr_pubkey == their_pubkey) {
        contacts.insert(0, contact.clone()); // Add to front
        save_contacts_to_store(&app, &contacts)?;
    }
    
    Ok(contact)
}

// ============================================================================
// Contact Management Commands
// ============================================================================

#[tauri::command]
pub fn get_contacts(app: AppHandle) -> Vec<Contact> {
    load_contacts_from_store(&app)
}

#[tauri::command]
pub fn delete_contact(id: String, app: AppHandle) -> Result<(), String> {
    let mut contacts = load_contacts_from_store(&app);
    contacts.retain(|c| c.id != id);
    save_contacts_to_store(&app, &contacts)
}
