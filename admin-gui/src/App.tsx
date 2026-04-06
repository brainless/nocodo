import { For, createSignal } from 'solid-js';

const menuItems = ['File', 'Edit', 'View', 'Insert', 'Format', 'Data', 'Tools', 'Help'];
const columns = Array.from({ length: 12 }, (_, index) => String.fromCharCode(65 + index));
const rows = Array.from({ length: 28 }, (_, index) => index + 1);
const sheets = ['Leads', 'Pipeline', 'Forecast', 'Invoices', 'Archive'];

export default function App() {
  const [messages, setMessages] = createSignal<{role: 'user' | 'assistant', content: string}[]>([
    { role: 'assistant', content: 'Hello! I can help you with your spreadsheet. What would you like to do?' }
  ]);
  const [inputValue, setInputValue] = createSignal('');

  const handleSend = () => {
    if (!inputValue().trim()) return;
    setMessages([...messages(), { role: 'user', content: inputValue() }]);
    setInputValue('');
    // Simulate AI response
    setTimeout(() => {
      setMessages(prev => [...prev, { 
        role: 'assistant', 
        content: 'I received your message. How else can I help you?' 
      }]);
    }, 500);
  };

  return (
    <main class="sheet-app">
      <header class="menu-strip">
        <div class="menu-title">Nocodo Sheets</div>
        <nav class="menu-items">
          <For each={menuItems}>{(item) => <button class="menu-item">{item}</button>}</For>
        </nav>
        <div class="menu-status">Synced</div>
      </header>

      <div class="drawer">
        <input id="chat-drawer" type="checkbox" class="drawer-toggle" />
        
        <div class="drawer-content flex flex-col">
          <section class="sheet-main">
            <div class="formula-strip">
              <label for="chat-drawer" class="chat-toggle-btn">
                Chat
              </label>
              <div class="name-box">B6</div>
              <label class="formula-label" for="formula-input">
                fx
              </label>
              <input
                id="formula-input"
                class="formula-input"
                value="=SUM(B2:B5)"
                aria-label="Formula bar"
              />
            </div>

            <div class="grid-wrap">
              <div class="grid-corner" />
              <For each={columns}>
                {(col) => <div class="column-header">{col}</div>}
              </For>

              <For each={rows}>
                {(row) => (
                  <>
                    <div class="row-header">{row}</div>
                    <For each={columns}>
                      {(col) => (
                        <div class={`cell${row === 6 && col === 'B' ? ' cell-active' : ''}`}>
                          {row === 1 ? `${col} Header` : ''}
                        </div>
                      )}
                    </For>
                  </>
                )}
              </For>
            </div>
          </section>

          <footer class="sheets-strip">
            <button class="sheet-add" aria-label="Add sheet">
              +
            </button>
            <div class="sheet-list">
              <For each={sheets}>
                {(sheet, index) => (
                  <button class={`sheet-tab${index() === 0 ? ' sheet-tab-active' : ''}`}>{sheet}</button>
                )}
              </For>
            </div>
          </footer>
        </div>

        <div class="drawer-side z-50">
          <label for="chat-drawer" aria-label="close sidebar" class="drawer-overlay"></label>
          <div class="chat-sidebar">
            <div class="chat-header">
              <h3>AI Assistant</h3>
              <label for="chat-drawer" class="chat-close-btn">✕</label>
            </div>
            
            <div class="chat-messages">
              <For each={messages()}>
                {(msg) => (
                  <div class={`chat-message ${msg.role === 'user' ? 'chat-message-user' : 'chat-message-assistant'}`}>
                    {msg.content}
                  </div>
                )}
              </For>
            </div>
            
            <div class="chat-input-area">
              <div class="chat-input-row">
                <input
                  type="text"
                  placeholder="Ask me anything..."
                  class="chat-input-field"
                  value={inputValue()}
                  onInput={(e) => setInputValue(e.currentTarget.value)}
                  onKeyDown={(e) => e.key === 'Enter' && handleSend()}
                />
                <button class="chat-send-btn" onClick={handleSend}>
                  Send
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
