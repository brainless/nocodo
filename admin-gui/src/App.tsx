import { For, createSignal } from 'solid-js';

const menuItems = ['File', 'Edit', 'View', 'Insert', 'Format', 'Data', 'Tools', 'Help'];
const columns = Array.from({ length: 12 }, (_, index) => String.fromCharCode(65 + index));
const rows = Array.from({ length: 28 }, (_, index) => index + 1);
const sheets = ['Leads', 'Pipeline', 'Forecast', 'Invoices', 'Archive'];

const API_BASE_URL = '';  // Use relative URLs to leverage Vite proxy
const PROJECT_ID = 1; // Default project for now

export default function App() {
  const [selectedCell, setSelectedCell] = createSignal({ col: 'B', row: 6 });
  // Store raw formulas/values for editing in the formula bar
  const [cellFormulas, setCellFormulas] = createSignal<Record<string, string>>({
    'A1': 'A Header',
    'B1': 'B Header',
    'C1': 'C Header',
    'D1': 'D Header',
    'E1': 'E Header',
    'F1': 'F Header',
    'G1': 'G Header',
    'H1': 'H Header',
    'I1': 'I Header',
    'J1': 'J Header',
    'K1': 'K Header',
    'L1': 'L Header',
    'B6': '=SUM(B2:B5)'
  });
  const [formulaValue, setFormulaValue] = createSignal('=SUM(B2:B5)');
  const [messages, setMessages] = createSignal<{role: 'user' | 'assistant', content: string}[]>([
    { role: 'assistant', content: 'Hello! I can help you with your spreadsheet. What would you like to do?' }
  ]);
  const [inputValue, setInputValue] = createSignal('');
  const [sessionId, setSessionId] = createSignal<number | null>(null);
  const [isLoading, setIsLoading] = createSignal(false);

  const getCellKey = (col: string, row: number) => `${col}${row}`;

  // Get the display value for a cell (computed value or the stored value)
  const getCellDisplayValue = (col: string, row: number): string => {
    const key = getCellKey(col, row);
    const formula = cellFormulas()[key];
    if (!formula) return '';
    // For now, just return the formula/value as-is (in a real app, this would compute formulas)
    return formula;
  };

  const handleCellClick = (col: string, row: number) => {
    setSelectedCell({ col, row });
    const key = getCellKey(col, row);
    setFormulaValue(cellFormulas()[key] || '');
  };

  const handleFormulaChange = (value: string) => {
    setFormulaValue(value);
    const key = getCellKey(selectedCell().col, selectedCell().row);
    setCellFormulas({ ...cellFormulas(), [key]: value });
  };

  const pollForResponse = async (messageId: number) => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/agents/schema-designer/messages/${messageId}/response`,
        { 
          method: 'GET',
          headers: { 'Content-Type': 'application/json' }
        }
      );
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      const data = await response.json();
      
      if (data.response && data.response.type === 'pending') {
        // Still pending, poll again after a short delay
        setTimeout(() => pollForResponse(messageId), 500);
        return;
      }
      
      // We have a response
      if (data.response) {
        const responseText = data.response.text || 'No response from agent';
        setMessages(prev => [...prev, { 
          role: 'assistant', 
          content: responseText 
        }]);
      } else {
        setMessages(prev => [...prev, { 
          role: 'assistant', 
          content: 'No response received' 
        }]);
      }
    } catch (error) {
      console.error('Error polling for response:', error);
      setMessages(prev => [...prev, { 
        role: 'assistant', 
        content: `Error: ${error instanceof Error ? error.message : 'Failed to get response'}` 
      }]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSend = async () => {
    const message = inputValue().trim();
    if (!message) return;
    
    // Add user message to UI immediately
    setMessages([...messages(), { role: 'user', content: message }]);
    setInputValue('');
    setIsLoading(true);
    
    try {
      // Send message to API
      const response = await fetch(`${API_BASE_URL}/api/agents/schema-designer/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          project_id: PROJECT_ID,
          session_id: sessionId(),
          message: message
        })
      });
      
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      
      const data = await response.json();
      
      // Store session_id if this is a new session
      if (data.session_id && !sessionId()) {
        setSessionId(data.session_id);
      }
      
      // Start polling for the response
      if (data.message_id) {
        pollForResponse(data.message_id);
      } else {
        setIsLoading(false);
        setMessages(prev => [...prev, { 
          role: 'assistant', 
          content: 'Error: No message ID received' 
        }]);
      }
    } catch (error) {
      console.error('Error sending message:', error);
      setIsLoading(false);
      setMessages(prev => [...prev, { 
        role: 'assistant', 
        content: `Error: ${error instanceof Error ? error.message : 'Failed to send message'}` 
      }]);
    }
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
              <div class="name-box">{selectedCell().col}{selectedCell().row}</div>
              <label class="formula-label" for="formula-input">
                fx
              </label>
              <input
                id="formula-input"
                class="formula-input"
                value={formulaValue()}
                onInput={(e) => handleFormulaChange(e.currentTarget.value)}
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
                        <div 
                          class={`cell${row === selectedCell().row && col === selectedCell().col ? ' cell-active' : ''}`}
                          onClick={() => handleCellClick(col, row)}
                        >
                          {getCellDisplayValue(col, row)}
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
                  onKeyDown={(e) => e.key === 'Enter' && !isLoading() && handleSend()}
                  disabled={isLoading()}
                />
                <button class="chat-send-btn" onClick={handleSend} disabled={isLoading()}>
                  {isLoading() ? '...' : 'Send'}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
