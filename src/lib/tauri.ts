import { invoke } from '@tauri-apps/api/core';
import type { Contact, NostrKeys, IrohStatus, ChatMessage } from './types';

// Key management commands
export async function hasKeys(): Promise<boolean> {
  return invoke<boolean>('has_keys');
}

export async function generateKeys(): Promise<NostrKeys> {
  return invoke<NostrKeys>('generate_keys');
}

export async function getPublicKey(): Promise<NostrKeys> {
  return invoke<NostrKeys>('get_public_key');
}

// NFC Exchange commands

// Start broadcasting our exchange message via NFC (sender mode)
export async function startNfcBroadcast(): Promise<string> {
  // Returns our pubkey after broadcasting
  return invoke<string>('start_nfc_broadcast');
}

// Start receiving/scanning for NFC exchange message (receiver mode)
export async function startNfcReceive(): Promise<string> {
  // Returns the received pubkey from NFC scan
  return invoke<string>('start_nfc_receive');
}

// Legacy alias for startNfcReceive
export async function startNfcScan(): Promise<string> {
  return invoke<string>('start_nfc_scan');
}

export async function writeNfcResponse(theirPubkey: string): Promise<void> {
  return invoke<void>('write_nfc_response', { theirPubkey });
}

export async function completeExchange(theirPubkey: string): Promise<Contact> {
  return invoke<Contact>('complete_exchange', { theirPubkey });
}

// QR Exchange commands
export async function getExchangeQrPayload(theirPubkey?: string): Promise<string> {
  return invoke<string>('get_exchange_qr_payload', { theirPubkey: theirPubkey ?? null });
}

export async function processScannedQr(qrData: string): Promise<string> {
  return invoke<string>('process_scanned_qr', { qrData });
}

// Contact management commands
export async function getContacts(): Promise<Contact[]> {
  return invoke<Contact[]>('get_contacts');
}

export async function deleteContact(id: string): Promise<void> {
  return invoke<void>('delete_contact', { id });
}

// Check NFC availability
export async function isNfcAvailable(): Promise<boolean> {
  return invoke<boolean>('is_nfc_available');
}

// Iroh chat commands
export async function startIroh(contactPubkey: string): Promise<IrohStatus> {
  return invoke<IrohStatus>('start_iroh', { contactPubkey });
}

export async function stopIroh(): Promise<void> {
  return invoke<void>('stop_iroh');
}

export async function getIrohStatus(): Promise<IrohStatus> {
  return invoke<IrohStatus>('get_iroh_status');
}

export async function connectToContact(contactPubkey: string, theirNodeId: string): Promise<void> {
  return invoke<void>('connect_to_contact', { contactPubkey, theirNodeId });
}

export async function sendMessage(contactPubkey: string, content: string): Promise<ChatMessage> {
  return invoke<ChatMessage>('send_message', { contactPubkey, content });
}

export async function getMessages(contactPubkey: string): Promise<ChatMessage[]> {
  return invoke<ChatMessage[]>('get_messages', { contactPubkey });
}
