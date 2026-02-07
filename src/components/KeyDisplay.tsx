import { type Component, Show } from 'solid-js';
import type { NostrKeys } from '../lib/types';

interface KeyDisplayProps {
  keys: NostrKeys | null;
}

const KeyDisplay: Component<KeyDisplayProps> = (props) => {
  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      // Could add a toast notification here
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  return (
    <div class="key-display">
      <h2>Your Identity</h2>
      
      <Show when={props.keys} fallback={
        <div class="card">
          <p>No keys generated yet</p>
        </div>
      }>
        {(keys) => (
          <>
            <div class="card">
              <div class="card-header">Nostr Public Key (npub)</div>
              <div 
                class="pubkey" 
                onClick={() => copyToClipboard(keys().publicKeyBech32)}
                style={{ cursor: 'pointer' }}
              >
                {keys().publicKeyBech32}
              </div>
              <p class="pubkey-label">Tap to copy</p>
            </div>

            <div class="card">
              <div class="card-header">Hex Format</div>
              <div 
                class="pubkey" 
                onClick={() => copyToClipboard(keys().publicKey)}
                style={{ cursor: 'pointer' }}
              >
                {keys().publicKey}
              </div>
              <p class="pubkey-label">Tap to copy</p>
            </div>

            <div class="card" style={{ "background": "transparent", "border": "1px solid var(--bg-card)" }}>
              <p style={{ "font-size": "14px", "color": "var(--text-secondary)", "line-height": "1.5" }}>
                This is your Nostr identity. Share it with others to let them find you on the Nostr network.
                Use the Exchange tab to securely exchange keys via NFC.
              </p>
            </div>
          </>
        )}
      </Show>
    </div>
  );
};

export default KeyDisplay;
