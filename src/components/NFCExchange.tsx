import { createSignal, type Component, Show } from 'solid-js';
import type { NostrKeys, Contact, ExchangeStatus } from '../lib/types';
import { startNfcScan, writeNfcResponse, completeExchange, isNfcAvailable } from '../lib/tauri';

interface NFCExchangeProps {
  keys: NostrKeys | null;
  onComplete: (contact: Contact) => void;
}

const NFCExchange: Component<NFCExchangeProps> = (props) => {
  const [status, setStatus] = createSignal<ExchangeStatus>({ state: 'idle' });
  const [nfcSupported, setNfcSupported] = createSignal<boolean | null>(null);

  const checkNfcSupport = async () => {
    try {
      const available = await isNfcAvailable();
      setNfcSupported(available);
    } catch {
      setNfcSupported(false);
    }
  };

  // Check NFC support on first interaction
  const startExchange = async () => {
    if (nfcSupported() === null) {
      await checkNfcSupport();
    }

    if (!nfcSupported()) {
      setStatus({ state: 'error', message: 'NFC is not available on this device' });
      return;
    }

    if (!props.keys) {
      setStatus({ state: 'error', message: 'No keys available' });
      return;
    }

    try {
      setStatus({ state: 'scanning' });

      // Step 1: Scan for the other device's pubkey
      const theirPubkey = await startNfcScan();
      setStatus({ state: 'received', theirPubkey });

      // Step 2: Write our signed response
      setStatus({ state: 'sending' });
      await writeNfcResponse(theirPubkey);

      // Step 3: Verify and complete the exchange
      setStatus({ state: 'verifying' });
      const contact = await completeExchange(theirPubkey);

      setStatus({ state: 'complete', contact });
      
      // Notify parent after a brief delay to show success
      setTimeout(() => {
        props.onComplete(contact);
        setStatus({ state: 'idle' });
      }, 2000);

    } catch (err) {
      console.error('Exchange error:', err);
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
      case 'scanning':
      case 'sending':
      case 'verifying':
        return (
          <svg class="nfc-icon scanning" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z" />
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
      case 'scanning':
        return 'Scanning...';
      case 'received':
        return 'Key Received!';
      case 'sending':
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
      case 'scanning':
        return 'Hold your device near another SneakerNet device';
      case 'received':
        return `Received pubkey: ${s.theirPubkey.slice(0, 16)}...`;
      case 'sending':
        return 'Writing signed response to NFC';
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
    return s.state === 'scanning' || s.state === 'received' || s.state === 'sending' || s.state === 'verifying';
  };

  return (
    <div class="nfc-exchange">
      {getStatusIcon()}
      
      <h2 class={`status-text ${status().state === 'complete' ? 'status-success' : ''} ${status().state === 'error' ? 'status-error' : ''}`}>
        {getStatusText()}
      </h2>
      
      <p class="status-detail">{getStatusDetail()}</p>

      <Show when={!isActive()}>
        <button 
          class="btn btn-primary" 
          onClick={startExchange}
          disabled={!props.keys}
        >
          {status().state === 'error' ? 'Try Again' : 'Start Exchange'}
        </button>
      </Show>

      <Show when={isActive()}>
        <button class="btn btn-secondary" onClick={cancelExchange}>
          Cancel
        </button>
      </Show>

      <Show when={status().state === 'complete'}>
        {(s) => {
          const state = s() as { state: 'complete'; contact: Contact };
          return (
            <div class="card" style={{ "margin-top": "24px", "text-align": "left" }}>
              <div class="card-header">New Contact</div>
              <div class="contact-pubkey">{state.contact.nostrPubkey}</div>
              <div class="contact-iroh">Iroh: {state.contact.irohEndpointId}</div>
            </div>
          );
        }}
      </Show>
    </div>
  );
};

export default NFCExchange;
