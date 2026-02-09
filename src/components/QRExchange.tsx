import { createSignal, onCleanup, type Component, Show } from 'solid-js';
import QRCode from 'qrcode';
import { scan, cancel, Format } from '@tauri-apps/plugin-barcode-scanner';
import type { NostrKeys, Contact, QRExchangeStatus } from '../lib/types';
import { getExchangeQrPayload, processScannedQr, completeExchange } from '../lib/tauri';

interface QRExchangeProps {
  keys: NostrKeys | null;
  onComplete: (contact: Contact) => void;
}

const QRExchange: Component<QRExchangeProps> = (props) => {
  const [status, setStatus] = createSignal<QRExchangeStatus>({ state: 'idle' });
  const [qrDataUrl, setQrDataUrl] = createSignal<string | null>(null);

  // Generate QR code when showing
  const generateQR = async (theirPk?: string) => {
    if (!props.keys) {
      setStatus({ state: 'error', message: 'No keys available' });
      return;
    }

    try {
      // Get the exchange payload
      const payload = await getExchangeQrPayload(theirPk);
      
      // Generate QR code
      const dataUrl = await QRCode.toDataURL(payload, {
        width: 280,
        margin: 2,
        color: {
          dark: '#1a1a2e',
          light: '#ffffff'
        }
      });
      
      setQrDataUrl(dataUrl);
      setStatus({ state: 'showing-qr' });
    } catch (err) {
      console.error('QR generation error:', err);
      setStatus({ 
        state: 'error', 
        message: err instanceof Error ? err.message : 'Failed to generate QR code' 
      });
    }
  };

  // Start scanning
  const startScanning = async () => {
    try {
      setStatus({ state: 'scanning' });
      
      // Use Tauri barcode scanner
      const result = await scan({
        windowed: false,
        formats: [Format.QRCode],
      });
      
      if (result.content) {
        // Process the scanned data
        setStatus({ state: 'processing', theirPubkey: '' });
        const pubkey = await processScannedQr(result.content);
        setStatus({ state: 'processing', theirPubkey: pubkey });
        
        // Complete the exchange
        const contact = await completeExchange(pubkey);
        setStatus({ state: 'complete', contact });
        
        // Show QR with their pubkey included (for mutual verification)
        await generateQR(pubkey);
        
        // Notify parent after a brief delay
        setTimeout(() => {
          props.onComplete(contact);
          reset();
        }, 3000);
      }
    } catch (err) {
      console.error('Scan error:', err);
      setStatus({ 
        state: 'error', 
        message: err instanceof Error ? err.message : 'Scan failed' 
      });
    }
  };

  // Cancel scanning
  const cancelScanning = async () => {
    try {
      await cancel();
    } catch {
      // Ignore cancel errors
    }
    setStatus({ state: 'idle' });
  };

  // Reset state
  const reset = () => {
    setStatus({ state: 'idle' });
    setQrDataUrl(null);
  };

  // Cleanup on unmount
  onCleanup(async () => {
    if (status().state === 'scanning') {
      try {
        await cancel();
      } catch {
        // Ignore
      }
    }
  });

  const getStatusIcon = () => {
    const s = status();
    switch (s.state) {
      case 'scanning':
      case 'processing':
        return (
          <svg class="qr-icon scanning" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <rect x="3" y="3" width="7" height="7" />
            <rect x="14" y="3" width="7" height="7" />
            <rect x="3" y="14" width="7" height="7" />
            <path d="M14 14h7v7h-7z" stroke-dasharray="2" />
          </svg>
        );
      case 'complete':
        return (
          <svg class="qr-icon" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2">
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
            <polyline points="22,4 12,14.01 9,11.01" />
          </svg>
        );
      case 'error':
        return (
          <svg class="qr-icon" viewBox="0 0 24 24" fill="none" stroke="var(--error)" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <line x1="15" y1="9" x2="9" y2="15" />
            <line x1="9" y1="9" x2="15" y2="15" />
          </svg>
        );
      default:
        return (
          <svg class="qr-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <rect x="3" y="3" width="7" height="7" />
            <rect x="14" y="3" width="7" height="7" />
            <rect x="3" y="14" width="7" height="7" />
            <rect x="14" y="14" width="7" height="7" />
          </svg>
        );
    }
  };

  const getStatusText = () => {
    const s = status();
    switch (s.state) {
      case 'idle':
        return 'QR Code Exchange';
      case 'showing-qr':
        return 'Show This QR Code';
      case 'scanning':
        return 'Scanning...';
      case 'processing':
        return 'Processing...';
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
        return 'Generate your QR code or scan another device';
      case 'showing-qr':
        return 'Have the other person scan this code, then scan theirs';
      case 'scanning':
        return 'Point camera at the other device\'s QR code';
      case 'processing':
        return s.theirPubkey ? `Processing: ${s.theirPubkey.slice(0, 16)}...` : 'Verifying...';
      case 'complete':
        return 'Contact added successfully';
      case 'error':
        return s.message;
    }
  };

  const isActive = () => {
    const s = status();
    return s.state === 'scanning' || s.state === 'processing';
  };

  return (
    <div class="qr-exchange">
      <Show when={status().state !== 'showing-qr'}>
        {getStatusIcon()}
      </Show>
      
      <Show when={qrDataUrl() && (status().state === 'showing-qr' || status().state === 'complete')}>
        <div class="qr-container">
          <img src={qrDataUrl()!} alt="QR Code" class="qr-image" />
        </div>
      </Show>
      
      <h2 class={`status-text ${status().state === 'complete' ? 'status-success' : ''} ${status().state === 'error' ? 'status-error' : ''}`}>
        {getStatusText()}
      </h2>
      
      <p class="status-detail">{getStatusDetail()}</p>

      <Show when={status().state === 'idle'}>
        <div class="qr-buttons">
          <button 
            class="btn btn-primary" 
            onClick={() => generateQR()}
            disabled={!props.keys}
          >
            Show My QR Code
          </button>
          <button 
            class="btn btn-secondary" 
            onClick={startScanning}
            disabled={!props.keys}
            style={{ "margin-top": "12px" }}
          >
            Scan QR Code
          </button>
        </div>
      </Show>

      <Show when={status().state === 'showing-qr'}>
        <div class="qr-buttons">
          <button 
            class="btn btn-primary" 
            onClick={startScanning}
          >
            Now Scan Their Code
          </button>
          <button 
            class="btn btn-secondary" 
            onClick={reset}
            style={{ "margin-top": "12px" }}
          >
            Cancel
          </button>
        </div>
      </Show>

      <Show when={isActive()}>
        <button class="btn btn-secondary" onClick={cancelScanning}>
          Cancel
        </button>
      </Show>

      <Show when={status().state === 'error'}>
        <button 
          class="btn btn-primary" 
          onClick={reset}
          style={{ "margin-top": "16px" }}
        >
          Try Again
        </button>
      </Show>

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

export default QRExchange;
