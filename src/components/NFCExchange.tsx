import { createSignal, type Component, Show } from 'solid-js';
import type { NostrKeys, Contact } from '../lib/types';
import { startNfcBroadcast, startNfcReceive, writeNfcResponse, completeExchange, isNfcAvailable } from '../lib/tauri';

interface NFCExchangeProps {
  keys: NostrKeys | null;
  onComplete: (contact: Contact) => void;
}

type NfcState = 
  | { state: 'idle' }
  | { state: 'choose-role' }  // New: choose sender or receiver
  | { state: 'sending' }       // Broadcasting our pubkey
  | { state: 'sent'; ourPubkey: string }  // Waiting for their response
  | { state: 'receiving' }     // Scanning for their pubkey
  | { state: 'received'; theirPubkey: string }  // Got their pubkey
  | { state: 'responding' }    // Writing our response
  | { state: 'verifying' }     // Completing exchange
  | { state: 'complete'; contact: Contact }
  | { state: 'error'; message: string };

const NFCExchange: Component<NFCExchangeProps> = (props) => {
  const [status, setStatus] = createSignal<NfcState>({ state: 'idle' });
  const [nfcSupported, setNfcSupported] = createSignal<boolean | null>(null);

  const checkNfcSupport = async () => {
    try {
      const available = await isNfcAvailable();
      setNfcSupported(available);
      return available;
    } catch {
      setNfcSupported(false);
      return false;
    }
  };

  // Start the exchange flow
  const startExchange = async () => {
    if (nfcSupported() === null) {
      const available = await checkNfcSupport();
      if (!available) {
        setStatus({ state: 'error', message: 'NFC is not available on this device' });
        return;
      }
    }

    if (!nfcSupported()) {
      setStatus({ state: 'error', message: 'NFC is not available on this device' });
      return;
    }

    if (!props.keys) {
      setStatus({ state: 'error', message: 'No keys available' });
      return;
    }

    // Show role selection
    setStatus({ state: 'choose-role' });
  };

  // SENDER FLOW: Write our pubkey first, then wait to receive their response
  const startAsSender = async () => {
    try {
      setStatus({ state: 'sending' });

      // Step 1: Broadcast our pubkey via NFC write
      const ourPubkey = await startNfcBroadcast();
      setStatus({ state: 'sent', ourPubkey });

      // Step 2: Now scan for their response (which should include our pubkey)
      setStatus({ state: 'receiving' });
      const theirPubkey = await startNfcReceive();
      setStatus({ state: 'received', theirPubkey });

      // Step 3: Complete the exchange
      setStatus({ state: 'verifying' });
      const contact = await completeExchange(theirPubkey);

      setStatus({ state: 'complete', contact });
      
      // Notify parent after a brief delay to show success
      setTimeout(() => {
        props.onComplete(contact);
        setStatus({ state: 'idle' });
      }, 2000);

    } catch (err) {
      console.error('NFC sender error:', err);
      setStatus({ 
        state: 'error', 
        message: err instanceof Error ? err.message : 'Exchange failed' 
      });
    }
  };

  // RECEIVER FLOW: Scan for their pubkey first, then write our response
  const startAsReceiver = async () => {
    try {
      // Step 1: Scan for their broadcast
      setStatus({ state: 'receiving' });
      const theirPubkey = await startNfcReceive();
      setStatus({ state: 'received', theirPubkey });

      // Step 2: Write our signed response (includes their pubkey)
      setStatus({ state: 'responding' });
      await writeNfcResponse(theirPubkey);

      // Step 3: Complete the exchange
      setStatus({ state: 'verifying' });
      const contact = await completeExchange(theirPubkey);

      setStatus({ state: 'complete', contact });
      
      // Notify parent after a brief delay to show success
      setTimeout(() => {
        props.onComplete(contact);
        setStatus({ state: 'idle' });
      }, 2000);

    } catch (err) {
      console.error('NFC receiver error:', err);
      setStatus({ 
        state: 'error', 
        message: err instanceof Error ? err.message : 'Exchange failed' 
      });
    }
  };

  const cancelExchange = () => {
    setStatus({ state: 'idle' });
  };

  const getStatusIcon = () => {
    const s = status();
    switch (s.state) {
      case 'choose-role':
        return (
          <svg class="nfc-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M6 8.32a7.43 7.43 0 0 1 0 7.36" />
            <path d="M9.46 6.21a11.76 11.76 0 0 1 0 11.58" />
            <path d="M12.91 4.1a15.91 15.91 0 0 1 .01 15.8" />
            <path d="M16.37 2a20.16 20.16 0 0 1 0 20" />
          </svg>
        );
      case 'sending':
      case 'receiving':
      case 'responding':
      case 'verifying':
        return (
          <svg class="nfc-icon scanning" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z" />
          </svg>
        );
      case 'sent':
      case 'received':
        return (
          <svg class="nfc-icon" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22,4 12,14.01 9,11.01" />
          </svg>
        );
      case 'complete':
        return (
          <svg class="nfc-icon" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22,4 12,14.01 9,11.01" />
          </svg>
        );
      case 'error':
        return (
          <svg class="nfc-icon" viewBox="0 0 24 24" fill="none" stroke="var(--error)" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <line x1="15" y1="9" x2="9" y2="15" />
            <line x1="9" y1="9" x2="15" y2="15" />
          </svg>
        );
      default:
        return (
          <svg class="nfc-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M6 8.32a7.43 7.43 0 0 1 0 7.36" />
            <path d="M9.46 6.21a11.76 11.76 0 0 1 0 11.58" />
            <path d="M12.91 4.1a15.91 15.91 0 0 1 .01 15.8" />
            <path d="M16.37 2a20.16 20.16 0 0 1 0 20" />
          </svg>
        );
    }
  };

  const getStatusText = () => {
    const s = status();
    switch (s.state) {
      case 'idle':
        return 'Ready to Exchange';
      case 'choose-role':
        return 'Choose Your Role';
      case 'sending':
        return 'Tap Devices Together';
      case 'sent':
        return 'Sent! Now Receiving...';
      case 'receiving':
        return 'Tap Devices Together';
      case 'received':
        return 'Key Received!';
      case 'responding':
        return 'Sending Response...';
      case 'verifying':
        return 'Verifying...';
      case 'complete':
        return 'Exchange Complete!';
      case 'error':
        return 'Exchange Failed';
    }
  };

  const getStatusDetail = () => {
    const s = status();
    switch (s.state) {
      case 'idle':
        return 'Tap another device to exchange keys securely';
      case 'choose-role':
        return 'One device should Send, the other should Receive';
      case 'sending':
        return 'Hold your device near the other phone to send your key';
      case 'sent':
        return 'Key sent! Now waiting to receive their key...';
      case 'receiving':
        return 'Hold your device near the other phone to receive their key';
      case 'received':
        return `Received: ${s.theirPubkey.slice(0, 16)}...`;
      case 'responding':
        return 'Sending your signed response...';
      case 'verifying':
        return 'Verifying signatures and deriving keys';
      case 'complete':
        return 'Contact added successfully';
      case 'error':
        return s.message;
    }
  };

  const isActive = () => {
    const s = status();
    return ['sending', 'sent', 'receiving', 'received', 'responding', 'verifying'].includes(s.state);
  };

  return (
    <div class="nfc-exchange">
      {getStatusIcon()}
      
      <h2 class={`status-text ${status().state === 'complete' ? 'status-success' : ''} ${status().state === 'error' ? 'status-error' : ''}`}>
        {getStatusText()}
      </h2>
      
      <p class="status-detail">{getStatusDetail()}</p>

      {/* Idle state - Start button */}
      <Show when={status().state === 'idle'}>
        <button 
          class="btn btn-primary" 
          onClick={startExchange}
          disabled={!props.keys}
        >
          Start NFC Exchange
        </button>
      </Show>

      {/* Role selection */}
      <Show when={status().state === 'choose-role'}>
        <div class="role-buttons">
          <button 
            class="btn btn-primary" 
            onClick={startAsSender}
          >
            Send First
          </button>
          <p class="role-hint">Choose this if the other device will Receive</p>
          
          <button 
            class="btn btn-secondary" 
            onClick={startAsReceiver}
            style={{ "margin-top": "16px" }}
          >
            Receive First
          </button>
          <p class="role-hint">Choose this if the other device will Send</p>
          
          <button 
            class="btn btn-text" 
            onClick={cancelExchange}
            style={{ "margin-top": "16px" }}
          >
            Cancel
          </button>
        </div>
      </Show>

      {/* Active state - Cancel button */}
      <Show when={isActive()}>
        <button class="btn btn-secondary" onClick={cancelExchange}>
          Cancel
        </button>
      </Show>

      {/* Error state - Try again */}
      <Show when={status().state === 'error'}>
        <button 
          class="btn btn-primary" 
          onClick={startExchange}
          style={{ "margin-top": "16px" }}
        >
          Try Again
        </button>
      </Show>

      {/* Complete state - Show contact details */}
      <Show when={status().state === 'complete' ? status() as { state: 'complete'; contact: Contact } : null}>
        {(state) => (
          <div class="card" style={{ "margin-top": "24px", "text-align": "left" }}>
            <div class="card-header">New Contact</div>
            <div class="contact-pubkey">{state().contact.nostrPubkey}</div>
            <div class="contact-iroh">Iroh: {state().contact.irohEndpointId}</div>
          </div>
        )}
      </Show>
    </div>
  );
};

export default NFCExchange;
