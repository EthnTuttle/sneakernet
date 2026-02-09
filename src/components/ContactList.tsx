import { type Component, For, Show } from 'solid-js';
import type { Contact } from '../lib/types';
import { deleteContact } from '../lib/tauri';

interface ContactListProps {
  contacts: Contact[];
  onRefresh: () => void;
  onOpenChat: (contact: Contact) => void;
}

const ContactList: Component<ContactListProps> = (props) => {
  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    return date.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const truncatePubkey = (pubkey: string) => {
    if (pubkey.length <= 20) return pubkey;
    return `${pubkey.slice(0, 10)}...${pubkey.slice(-10)}`;
  };

  const handleDelete = async (id: string) => {
    if (confirm('Delete this contact?')) {
      try {
        await deleteContact(id);
        props.onRefresh();
      } catch (err) {
        console.error('Failed to delete contact:', err);
      }
    }
  };

  return (
    <div>
      <div style={{ display: 'flex', "justify-content": 'space-between', "align-items": 'center', "margin-bottom": '16px' }}>
        <h2>Contacts</h2>
        <span style={{ color: 'var(--text-secondary)', "font-size": '14px' }}>
          {props.contacts.length} {props.contacts.length === 1 ? 'contact' : 'contacts'}
        </span>
      </div>

      <Show when={props.contacts.length > 0} fallback={
        <div class="empty-state">
          <div class="empty-state-icon">
            <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
              <circle cx="9" cy="7" r="4" />
              <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
              <path d="M16 3.13a4 4 0 0 1 0 7.75" />
            </svg>
          </div>
          <h3>No contacts yet</h3>
          <p style={{ "margin-top": '8px' }}>
            Use the Exchange tab to add contacts via NFC
          </p>
        </div>
      }>
        <ul class="contact-list">
          <For each={props.contacts}>
            {(contact) => (
              <li class="contact-item">
                <div class="contact-pubkey">
                  <strong>Nostr:</strong> {truncatePubkey(contact.nostrPubkey)}
                </div>
                <div class="contact-iroh">
                  <strong>Iroh:</strong> {truncatePubkey(contact.irohEndpointId)}
                </div>
                <div class="contact-actions">
                  <button 
                    class="chat-button"
                    onClick={() => props.onOpenChat(contact)}
                  >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                      <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
                    </svg>
                    Chat
                  </button>
                </div>
                <div class="contact-meta">
                  <span>{formatDate(contact.exchangedAt)}</span>
                  <button 
                    onClick={() => handleDelete(contact.id)}
                    style={{ 
                      background: 'none', 
                      border: 'none', 
                      color: 'var(--error)', 
                      cursor: 'pointer',
                      padding: '4px 8px',
                      "font-size": '12px'
                    }}
                  >
                    Delete
                  </button>
                </div>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
};

export default ContactList;
