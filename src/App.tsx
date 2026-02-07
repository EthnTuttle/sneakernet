import { createSignal, onMount, Show, type Component } from 'solid-js';
import KeyDisplay from './components/KeyDisplay';
import NFCExchange from './components/NFCExchange';
import ContactList from './components/ContactList';
import type { TabId, NostrKeys, Contact } from './lib/types';
import { hasKeys, generateKeys, getPublicKey, getContacts } from './lib/tauri';

const App: Component = () => {
  const [activeTab, setActiveTab] = createSignal<TabId>('keys');
  const [keys, setKeys] = createSignal<NostrKeys | null>(null);
  const [contacts, setContacts] = createSignal<Contact[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      // Check if keys exist, generate if not
      const keysExist = await hasKeys();
      
      if (keysExist) {
        const existingKeys = await getPublicKey();
        setKeys(existingKeys);
      } else {
        const newKeys = await generateKeys();
        setKeys(newKeys);
      }

      // Load contacts
      const savedContacts = await getContacts();
      setContacts(savedContacts);
    } catch (err) {
      console.error('Initialization error:', err);
      setError(err instanceof Error ? err.message : 'Failed to initialize');
    } finally {
      setLoading(false);
    }
  });

  const refreshContacts = async () => {
    try {
      const savedContacts = await getContacts();
      setContacts(savedContacts);
    } catch (err) {
      console.error('Failed to refresh contacts:', err);
    }
  };

  const onExchangeComplete = (contact: Contact) => {
    setContacts(prev => [contact, ...prev]);
    setActiveTab('contacts');
  };

  return (
    <div class="app">
      <Show when={!loading()} fallback={
        <div class="loading">
          <div class="spinner" />
        </div>
      }>
        <Show when={!error()} fallback={
          <div class="content">
            <div class="card">
              <p class="status-error">Error: {error()}</p>
            </div>
          </div>
        }>
          <div class="content">
            <Show when={activeTab() === 'keys'}>
              <KeyDisplay keys={keys()} />
            </Show>
            
            <Show when={activeTab() === 'exchange'}>
              <NFCExchange 
                keys={keys()} 
                onComplete={onExchangeComplete} 
              />
            </Show>
            
            <Show when={activeTab() === 'contacts'}>
              <ContactList 
                contacts={contacts()} 
                onRefresh={refreshContacts}
              />
            </Show>
          </div>

          <nav class="tab-bar">
            <button 
              class={`tab-item ${activeTab() === 'keys' ? 'active' : ''}`}
              onClick={() => setActiveTab('keys')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
              </svg>
              Keys
            </button>
            
            <button 
              class={`tab-item ${activeTab() === 'exchange' ? 'active' : ''}`}
              onClick={() => setActiveTab('exchange')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z" />
              </svg>
              Exchange
            </button>
            
            <button 
              class={`tab-item ${activeTab() === 'contacts' ? 'active' : ''}`}
              onClick={() => setActiveTab('contacts')}
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
                <circle cx="9" cy="7" r="4" />
                <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                <path d="M16 3.13a4 4 0 0 1 0 7.75" />
              </svg>
              Contacts
            </button>
          </nav>
        </Show>
      </Show>
    </div>
  );
};

export default App;
