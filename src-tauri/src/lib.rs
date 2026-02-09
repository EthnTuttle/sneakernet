//! SneakerNet - NFC/QR-based cryptographic key exchange with Iroh p2p chat
//!
//! This is the Tauri backend for the SneakerNet Android app.
//! It handles Nostr key management, NFC/QR exchange protocol, Iroh key derivation,
//! and p2p chat functionality.

pub mod chat;
pub mod commands;
pub mod exchange;
pub mod iroh_derive;
pub mod iroh_node;
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
                _app.handle().plugin(tauri_plugin_barcode_scanner::init())?;
            }
            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            // Key management
            commands::has_keys,
            commands::generate_keys,
            commands::get_public_key,
            // NFC exchange
            commands::is_nfc_available,
            commands::start_nfc_broadcast,
            commands::start_nfc_receive,
            commands::start_nfc_scan, // Legacy alias for start_nfc_receive
            commands::write_nfc_response,
            commands::complete_exchange,
            // QR exchange
            commands::get_exchange_qr_payload,
            commands::process_scanned_qr,
            // Contact management
            commands::get_contacts,
            commands::delete_contact,
            // Iroh chat
            commands::start_iroh,
            commands::stop_iroh,
            commands::get_iroh_status,
            commands::connect_to_contact,
            commands::send_message,
            commands::get_messages,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
