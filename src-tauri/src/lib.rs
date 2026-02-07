//! SneakerNet - NFC-based cryptographic key exchange
//!
//! This is the Tauri backend for the SneakerNet Android app.
//! It handles Nostr key management, NFC exchange protocol, and Iroh key derivation.

pub mod commands;
pub mod exchange;
pub mod iroh_derive;
pub mod keys;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|_app| {
            #[cfg(mobile)]
            {
                _app.handle().plugin(tauri_plugin_nfc::init())?;
            }
            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::has_keys,
            commands::generate_keys,
            commands::get_public_key,
            commands::is_nfc_available,
            commands::start_nfc_scan,
            commands::write_nfc_response,
            commands::complete_exchange,
            commands::get_contacts,
            commands::delete_contact,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
