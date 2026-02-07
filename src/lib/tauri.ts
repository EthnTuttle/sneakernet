import { invoke } from '@tauri-apps/api/core';
import type { Contact, NostrKeys } from './types';

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
export async function startNfcScan(): Promise<string> {
  // Returns the received pubkey from NFC scan
  return invoke<string>('start_nfc_scan');
}

export async function writeNfcResponse(theirPubkey: string): Promise<void> {
  return invoke<void>('write_nfc_response', { theirPubkey });
}

export async function completeExchange(theirPubkey: string): Promise<Contact> {
  return invoke<Contact>('complete_exchange', { theirPubkey });
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
