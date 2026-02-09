import { createSignal, createEffect, onMount, onCleanup, type Component, For, Show } from 'solid-js';
import type { Contact, ChatMessage, IrohStatus } from '../lib/types';
import { 
  startIroh, 
  connectToContact, 
  sendMessage, 
  getMessages 
} from '../lib/tauri';

interface ChatProps {
  contact: Contact;
  onBack: () => void;
}

const Chat: Component<ChatProps> = (props) => {
  const [messages, setMessages] = createSignal<ChatMessage[]>([]);
  const [newMessage, setNewMessage] = createSignal('');
  const [irohStatus, setIrohStatus] = createSignal<IrohStatus | null>(null);
  const [connecting, setConnecting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [sending, setSending] = createSignal(false);
  
  let messagesEndRef: HTMLDivElement | undefined;
  let pollInterval: number | undefined;

  // Scroll to bottom when new messages arrive
  createEffect(() => {
    const msgs = messages();
    if (messagesEndRef && msgs.length > 0) {
      messagesEndRef.scrollIntoView({ behavior: 'smooth' });
    }
  });

  onMount(async () => {
    await initializeChat();
  });

  onCleanup(async () => {
    if (pollInterval) {
      clearInterval(pollInterval);
    }
  });

  const initializeChat = async () => {
    try {
      setConnecting(true);
      setError(null);

      // Start Iroh for this contact
      const status = await startIroh(props.contact.nostrPubkey);
      setIrohStatus(status);

      // Try to connect to the contact
      await connectToContact(props.contact.nostrPubkey, props.contact.irohEndpointId);

      // Load existing messages
      const existingMessages = await getMessages(props.contact.nostrPubkey);
      setMessages(existingMessages);

      // Start polling for new messages
      pollInterval = setInterval(async () => {
        try {
          const newMessages = await getMessages(props.contact.nostrPubkey);
          setMessages(newMessages);
        } catch (err) {
          console.error('Failed to poll messages:', err);
        }
      }, 2000) as unknown as number;

      setConnecting(false);
    } catch (err) {
      console.error('Chat initialization error:', err);
      setError(err instanceof Error ? err.message : 'Failed to connect');
      setConnecting(false);
    }
  };

  const handleSend = async () => {
    const content = newMessage().trim();
    if (!content || sending()) return;

    try {
      setSending(true);
      const msg = await sendMessage(props.contact.nostrPubkey, content);
      setMessages(prev => [...prev, msg]);
      setNewMessage('');
    } catch (err) {
      console.error('Send error:', err);
      setError(err instanceof Error ? err.message : 'Failed to send');
    } finally {
      setSending(false);
    }
  };

  const handleKeyPress = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    return date.toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const truncatePubkey = (pubkey: string) => {
    if (pubkey.length <= 16) return pubkey;
    return `${pubkey.slice(0, 8)}...${pubkey.slice(-8)}`;
  };

  return (
    <div class="chat-container">
      <div class="chat-header">
        <button class="back-button" onClick={props.onBack}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M19 12H5M12 19l-7-7 7-7" />
          </svg>
        </button>
        <div class="chat-header-info">
          <h3>{props.contact.nickname || truncatePubkey(props.contact.nostrPubkey)}</h3>
          <Show when={irohStatus()}>
            <span class="connection-status">
              {irohStatus()!.running ? '[o] Connected' : '[x] Disconnected'}
            </span>
          </Show>
        </div>
      </div>

      <Show when={connecting()}>
        <div class="chat-connecting">
          <div class="spinner" />
          <p>Connecting to peer...</p>
        </div>
      </Show>

      <Show when={error()}>
        <div class="chat-error">
          <p>{error()}</p>
          <button class="btn btn-secondary" onClick={initializeChat}>
            Retry
          </button>
        </div>
      </Show>

      <Show when={!connecting() && !error()}>
        <div class="chat-messages">
          <Show when={messages().length === 0}>
            <div class="chat-empty">
              <p>No messages yet</p>
              <p class="chat-empty-hint">Send a message to start the conversation</p>
            </div>
          </Show>
          
          <For each={messages()}>
            {(msg) => (
              <div class={`message ${msg.isOutgoing ? 'outgoing' : 'incoming'}`}>
                <div class="message-content">{msg.content}</div>
                <div class="message-time">{formatTime(msg.timestamp)}</div>
              </div>
            )}
          </For>
          <div ref={messagesEndRef} />
        </div>

        <div class="chat-input-container">
          <input
            type="text"
            class="chat-input"
            placeholder="Type a message..."
            value={newMessage()}
            onInput={(e) => setNewMessage(e.currentTarget.value)}
            onKeyPress={handleKeyPress}
            disabled={sending()}
          />
          <button 
            class="send-button" 
            onClick={handleSend}
            disabled={!newMessage().trim() || sending()}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="22" y1="2" x2="11" y2="13" />
              <polygon points="22,2 15,22 11,13 2,9" />
            </svg>
          </button>
        </div>
      </Show>
    </div>
  );
};

export default Chat;
