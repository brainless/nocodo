import { For, Show, createSignal, onMount } from 'solid-js';
import type { 
  Sheet, 
  SheetTab, 
  SheetTabColumn, 
  SheetTabRow,
  ListSheetsResponse, 
  GetSheetResponse, 
  GetSheetTabSchemaResponse,
  GetSheetTabDataResponse,
  ColumnType 
} from './types/api';

const menuItems = ['File', 'Edit', 'View', 'Insert', 'Format', 'Data', 'Tools', 'Help'];
const columns = Array.from({ length: 12 }, (_, index) => String.fromCharCode(65 + index));
const rows = Array.from({ length: 28 }, (_, index) => index + 1);

const API_BASE_URL = '';  // Use relative URLs to leverage Vite proxy
const PROJECT_ID = 1; // Default project for now

type ChatRole = 'user' | 'assistant';
type UiMessage = { role: ChatRole; content: string };
type HistoryMessage = {
  id: number;
  role: string;
  content: string;
  created_at: number;
};

const defaultAssistantMessage: UiMessage = {
  role: 'assistant',
  content: 'Hello! I can help you with your spreadsheet. What would you like to do?'
};

export default function App() {
  // Sheet state
  const [sheets, setSheets] = createSignal<Sheet[]>([]);
  const [currentSheet, setCurrentSheet] = createSignal<Sheet | null>(null);
  const [sheetTabs, setSheetTabs] = createSignal<SheetTab[]>([]);
  const [activeTabId, setActiveTabId] = createSignal<number | null>(null);
  const [columns, setColumns] = createSignal<SheetTabColumn[]>([]);
  const [rows, setRows] = createSignal<SheetTabRow[]>([]);
  const [isLoadingSheets, setIsLoadingSheets] = createSignal(false);
  const [sheetError, setSheetError] = createSignal<string | null>(null);

  // Cell/Grid state
  const [selectedCell, setSelectedCell] = createSignal({ col: 'A', row: 1 });
  const [cellFormulas, setCellFormulas] = createSignal<Record<string, string>>({});
  const [formulaValue, setFormulaValue] = createSignal('');

  // Chat state
  const [messages, setMessages] = createSignal<UiMessage[]>([defaultAssistantMessage]);
  const [inputValue, setInputValue] = createSignal('');
  const [sessionId, setSessionId] = createSignal<number | null>(null);
  const [isLoading, setIsLoading] = createSignal(false);

  // Fetch sheets on mount
  onMount(() => {
    loadSheets();
  });

  // Load sheets for the project
  const loadSheets = async () => {
    setIsLoadingSheets(true);
    setSheetError(null);
    try {
      const response = await fetch(`${API_BASE_URL}/api/sheets?project_id=${PROJECT_ID}`);
      if (!response.ok) {
        throw new Error(`Failed to load sheets: ${response.status}`);
      }
      const data = await response.json() as ListSheetsResponse;
      setSheets(data.sheets);
      
      // Load the first sheet if available
      if (data.sheets.length > 0) {
        await loadSheet(data.sheets[0].id);
      }
    } catch (error) {
      console.error('Error loading sheets:', error);
      setSheetError(error instanceof Error ? error.message : 'Failed to load sheets');
    } finally {
      setIsLoadingSheets(false);
    }
  };

  // Load a specific sheet with its tabs
  const loadSheet = async (sheetId: number) => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/sheets/${sheetId}`);
      if (!response.ok) {
        throw new Error(`Failed to load sheet: ${response.status}`);
      }
      const data = await response.json() as GetSheetResponse;
      setCurrentSheet(data.sheet);
      setSheetTabs(data.sheet_tabs);
      
      // Load the first tab if available
      if (data.sheet_tabs.length > 0) {
        await loadTab(data.sheet_tabs[0].id);
      }
    } catch (error) {
      console.error('Error loading sheet:', error);
      setSheetError(error instanceof Error ? error.message : 'Failed to load sheet');
    }
  };

  // Load a tab's schema and data
  const loadTab = async (tabId: number) => {
    setActiveTabId(tabId);
    try {
      // Load schema (columns)
      const schemaResponse = await fetch(`${API_BASE_URL}/api/sheet-tabs/${tabId}/schema`);
      if (!schemaResponse.ok) {
        throw new Error(`Failed to load tab schema: ${schemaResponse.status}`);
      }
      const schemaData = await schemaResponse.json() as GetSheetTabSchemaResponse;
      setColumns(schemaData.columns);

      // Load row data
      const dataResponse = await fetch(`${API_BASE_URL}/api/sheet-tabs/${tabId}/data?limit=100`);
      if (!dataResponse.ok) {
        throw new Error(`Failed to load tab data: ${dataResponse.status}`);
      }
      const dataData = await dataResponse.json() as GetSheetTabDataResponse;
      setRows(dataData.rows);
    } catch (error) {
      console.error('Error loading tab:', error);
      setSheetError(error instanceof Error ? error.message : 'Failed to load tab');
    }
  };

  // Get display value for a cell from the row data (rowIndex is 0-based data row)
  const getCellDisplayValue = (colIndex: number, rowIndex: number): string => {
    const cols = columns();
    const rowData = rows()[rowIndex]; // rowIndex 0 = displayed row 2
    
    if (!rowData || colIndex >= cols.length) return '';
    
    const column = cols[colIndex];
    const data = JSON.parse(rowData.data || '{}');
    const value = data[String(column.id)];
    
    if (value === null || value === undefined) return '';
    return String(value);
  };

  // Get the raw cell value for the formula bar
  const getCellValue = (col: string, row: number): string => {
    const colIndex = col.charCodeAt(0) - 65; // Convert 'A' -> 0, 'B' -> 1, etc.
    const cols = columns();
    
    // First check if there's a user-entered formula
    const key = getCellKey(col, row);
    const formula = cellFormulas()[key];
    if (formula !== undefined && formula !== '') {
      return formula;
    }
    
    // Row 1: show column names from schema
    if (row === 1 && colIndex >= 0 && colIndex < cols.length) {
      return cols[colIndex].name;
    }
    
    // Data rows (2+): get value from API row data (row 2 = index 0)
    const rowIndex = row - 2;
    const rowData = rows()[rowIndex];
    
    if (!rowData || colIndex < 0 || colIndex >= cols.length) return '';
    
    const column = cols[colIndex];
    const data = JSON.parse(rowData.data || '{}');
    const value = data[String(column.id)];
    
    if (value === null || value === undefined) return '';
    return String(value);
  };

  // Get column header name
  const getColumnHeader = (colIndex: number): string => {
    const cols = columns();
    if (colIndex < cols.length) {
      return cols[colIndex].name;
    }
    return String.fromCharCode(65 + colIndex);
  };

  // Check if a column is a relation type
  const isRelationColumn = (colIndex: number): boolean => {
    const cols = columns();
    if (colIndex >= cols.length) return false;
    const colType = cols[colIndex].column_type;
    return typeof colType === 'object' && colType !== null && 'type' in colType && colType.type === 'relation';
  };

  const getCellKey = (col: string, row: number) => `${col}${row}`;

  const handleCellClick = (col: string, row: number) => {
    setSelectedCell({ col, row });
    setFormulaValue(getCellValue(col, row));
  };

  const handleFormulaChange = (value: string) => {
    setFormulaValue(value);
    const key = getCellKey(selectedCell().col, selectedCell().row);
    setCellFormulas(prev => ({ ...prev, [key]: value }));
  };

  const loadSessionHistory = async (targetSessionId: number) => {
    const response = await fetch(
      `${API_BASE_URL}/api/agents/schema-designer/sessions/${targetSessionId}/messages`,
      {
        method: 'GET',
        headers: { 'Content-Type': 'application/json' }
      }
    );

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const data = await response.json() as { messages?: HistoryMessage[] };
    const history = (data.messages || [])
      .filter((msg) => msg.role === 'user' || msg.role === 'assistant')
      .map((msg) => ({ role: msg.role as ChatRole, content: msg.content }));

    setMessages(history.length > 0 ? history : [defaultAssistantMessage]);
  };

  const pollForResponse = async (messageId: number, targetSessionId: number) => {
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
        setTimeout(() => pollForResponse(messageId, targetSessionId), 500);
        return;
      }
      
      await loadSessionHistory(targetSessionId);
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
    
    setMessages([...messages(), { role: 'user', content: message }]);
    setInputValue('');
    setIsLoading(true);
    
    try {
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
      
      if (data.session_id && data.session_id !== sessionId()) {
        setSessionId(data.session_id);
      }
      
      if (data.message_id) {
        const targetSessionId = data.session_id || sessionId();
        if (!targetSessionId) {
          throw new Error('No session ID received');
        }
        pollForResponse(data.message_id, targetSessionId);
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
        <div class="menu-title">
          {currentSheet() ? currentSheet()!.name : 'Nocodo Sheets'}
        </div>
        <nav class="menu-items">
          <For each={menuItems}>{(item) => <button class="menu-item">{item}</button>}</For>
        </nav>
        <div class="menu-status">
          <Show when={isLoadingSheets()}>
            <span class="loading loading-spinner loading-xs mr-2"></span>
          </Show>
          {sheetError() ? 'Error' : 'Synced'}
        </div>
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

            <Show when={sheetError()}>
              <div class="alert alert-error m-4">
                <span>{sheetError()}</span>
                <button class="btn btn-sm btn-ghost" onClick={loadSheets}>Retry</button>
              </div>
            </Show>

            <div class="grid-wrap">
              <div class="grid-corner" />
              {/* Column headers: Always A, B, C... Z (26 columns) */}
              <For each={Array.from({ length: 26 }, (_, i) => i)}>
                {(i) => (
                  <div class="column-header">
                    {String.fromCharCode(65 + i)}
                  </div>
                )}
              </For>

              {/* Row 1: Column names from API */}
              <Show when={columns().length > 0}>
                <div class="row-header">1</div>
                <For each={columns()}>
                  {(col, colIndex) => {
                    const colLetter = String.fromCharCode(65 + colIndex());
                    return (
                      <div 
                        class={`cell${1 === selectedCell().row && colLetter === selectedCell().col ? ' cell-active' : ''}`}
                        onClick={() => handleCellClick(colLetter, 1)}
                      >
                        {col.name}
                      </div>
                    );
                  }}
                </For>
                {/* Fill remaining columns in header row up to Z */}
                <For each={columns().length < 26 ? Array.from({ length: 26 - columns().length }, (_, i) => i) : []}>
                  {(i) => {
                    const colLetter = String.fromCharCode(65 + columns().length + i);
                    return (
                      <div 
                        class={`cell${1 === selectedCell().row && colLetter === selectedCell().col ? ' cell-active' : ''}`}
                        onClick={() => handleCellClick(colLetter, 1)}
                      />
                    );
                  }}
                </For>
              </Show>

              {/* Data rows from API (starting from row 2) */}
              <For each={rows()}>
                {(row, rowIndex) => (
                  <>
                    <div class="row-header">{rowIndex() + 2}</div>
                    <For each={columns()}>
                      {(col, colIndex) => {
                        const colLetter = String.fromCharCode(65 + colIndex());
                        return (
                          <div 
                            class={`cell${rowIndex() + 2 === selectedCell().row && colLetter === selectedCell().col ? ' cell-active' : ''}`}
                            classList={{
                              'text-blue-600 underline cursor-pointer': isRelationColumn(colIndex())
                            }}
                            onClick={() => handleCellClick(colLetter, rowIndex() + 2)}
                            title={isRelationColumn(colIndex()) ? 'Click to view related record' : undefined}
                          >
                            {getCellDisplayValue(colIndex(), rowIndex())}
                          </div>
                        );
                      }}
                    </For>
                    {/* Fill remaining columns up to Z */}
                    <For each={columns().length < 26 ? Array.from({ length: 26 - columns().length }, (_, i) => i) : []}>
                      {(i) => {
                        const colLetter = String.fromCharCode(65 + columns().length + i);
                        return (
                          <div 
                            class={`cell${rowIndex() + 2 === selectedCell().row && colLetter === selectedCell().col ? ' cell-active' : ''}`}
                            onClick={() => handleCellClick(colLetter, rowIndex() + 2)}
                          />
                        );
                      }}
                    </For>
                  </>
                )}
              </For>

              {/* Empty rows to fill up to 100 total rows */}
              <Show when={columns().length === 0 || rows().length < 99}>
                <For each={Array.from({ length: columns().length > 0 ? 99 - rows().length : 100 }, (_, i) => i)}>
                  {(rowOffset) => {
                    // Start from row 2 if we have columns (row 1 has column names), else row 1
                    const startRow = columns().length > 0 ? 2 : 1;
                    const rowNum = rowOffset + startRow + rows().length;
                    return (
                      <>
                        <div class="row-header">{rowNum}</div>
                        <For each={Array.from({ length: 26 }, (_, i) => i)}>
                          {(colIndex) => {
                            const colLetter = String.fromCharCode(65 + colIndex);
                            return (
                              <div 
                                class={`cell${rowNum === selectedCell().row && colLetter === selectedCell().col ? ' cell-active' : ''}`}
                                onClick={() => handleCellClick(colLetter, rowNum)}
                              />
                            );
                          }}
                        </For>
                      </>
                    );
                  }}
                </For>
              </Show>
            </div>
          </section>

          <footer class="sheets-strip">
            <button class="sheet-add" aria-label="Add sheet">
              +
            </button>
            <div class="sheet-list">
              <Show when={sheetTabs().length === 0}>
                <span class="text-sm text-gray-500 px-2">No tabs available</span>
              </Show>
              <For each={sheetTabs()}>
                {(tab) => (
                  <button 
                    class={`sheet-tab${tab.id === activeTabId() ? ' sheet-tab-active' : ''}`}
                    onClick={() => loadTab(tab.id)}
                  >
                    {tab.name}
                  </button>
                )}
              </For>
            </div>
          </footer>
        </div>

        <div class="drawer-side z-50">
          <label for="chat-drawer" aria-label="close sidebar" class="drawer-overlay"></label>
          <div class="chat-sidebar">
            <div class="chat-panel-header">
              <h3 class="text-sm font-semibold">AI Assistant</h3>
              <label for="chat-drawer" class="btn btn-ghost btn-sm btn-square">✕</label>
            </div>
            
            <div class="chat-messages">
              <For each={messages()}>
                {(msg) => (
                  <div class={`chat ${msg.role === 'user' ? 'chat-end' : 'chat-start'}`}>
                    <div class={`chat-bubble whitespace-pre-wrap ${msg.role === 'user' ? 'chat-bubble-primary' : ''}`}>
                      {msg.content}
                    </div>
                  </div>
                )}
              </For>
            </div>
            
            <div class="chat-input-area">
              <div class="chat-input-row">
                <input
                  type="text"
                  placeholder="Ask me anything..."
                  class="input input-bordered input-sm chat-input-field"
                  value={inputValue()}
                  onInput={(e) => setInputValue(e.currentTarget.value)}
                  onKeyDown={(e) => e.key === 'Enter' && !isLoading() && handleSend()}
                  disabled={isLoading()}
                />
                <button class="btn btn-primary btn-sm chat-send-btn" onClick={handleSend} disabled={isLoading()}>
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
