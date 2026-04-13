import { For, Show, createSignal, onMount } from 'solid-js';
import { ProjectProvider, useProject } from './contexts/ProjectContext';
import type {
  Sheet,
  SheetTab,
  SheetTabColumn,
  ListSheetsResponse,
  GetSheetResponse,
  GetSheetTabSchemaResponse,
  GetSheetDataResponse,
  SheetTabDataResult,
  ColumnType
} from './types/api';

const menuItems: string[] = [];

const API_BASE_URL = '';  // Use relative URLs to leverage Vite proxy

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

// Project Selector Component
function ProjectSelector() {
  const { projects, currentProject, setCurrentProject, isLoading, createProject } = useProject();
  const [isDropdownOpen, setIsDropdownOpen] = createSignal(false);
  const [isModalOpen, setIsModalOpen] = createSignal(false);
  const [newProjectName, setNewProjectName] = createSignal('');
  const [isCreating, setIsCreating] = createSignal(false);

  const handleCreateProject = async () => {
    const name = newProjectName().trim();
    if (!name) return;
    
    setIsCreating(true);
    const project = await createProject(name);
    setIsCreating(false);
    
    if (project) {
      setNewProjectName('');
      setIsModalOpen(false);
    }
  };

  return (
    <>
      {/* Project Selector Dropdown */}
      <div class="dropdown dropdown-bottom">
        <button
          class="btn btn-ghost btn-sm gap-2"
          onClick={() => setIsDropdownOpen(!isDropdownOpen())}
        >
          <Show when={isLoading()}>
            <span class="loading loading-spinner loading-xs"></span>
          </Show>
          <span class="font-semibold">{currentProject()?.name ?? 'Select Project'}</span>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>
        </button>
        
        <Show when={isDropdownOpen()}>
          <div class="dropdown-content z-[100] menu p-2 shadow bg-base-100 rounded-box w-56 mt-2 border border-base-300">
            <div class="px-3 py-2 text-xs font-semibold text-base-content/60 uppercase tracking-wider">
              Projects
            </div>
            <For each={projects()}>
              {(project) => (
                <button
                  class={`btn btn-ghost btn-sm justify-start ${project.id === currentProject()?.id ? 'btn-active' : ''}`}
                  onClick={() => {
                    setCurrentProject(project);
                    setIsDropdownOpen(false);
                  }}
                >
                  <span class="truncate">{project.name}</span>
                </button>
              )}
            </For>
            
            <div class="divider my-1"></div>
            
            <button
              class="btn btn-ghost btn-sm justify-start text-primary"
              onClick={() => {
                setIsDropdownOpen(false);
                setIsModalOpen(true);
              }}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-1"><path d="M5 12h14"/><path d="M12 5v14"/></svg>
              Create Project
            </button>
          </div>
        </Show>
        
        {/* Backdrop to close dropdown */}
        <Show when={isDropdownOpen()}>
          <div 
            class="fixed inset-0 z-[99]" 
            onClick={() => setIsDropdownOpen(false)}
          />
        </Show>
      </div>

      {/* Create Project Modal */}
      <Show when={isModalOpen()}>
        <div class="modal modal-open z-[200]">
          <div class="modal-box">
            <h3 class="font-bold text-lg mb-4">Create New Project</h3>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">Project Name</span>
              </label>
              <input
                type="text"
                placeholder="Enter project name"
                class="input input-bordered"
                value={newProjectName()}
                onInput={(e) => setNewProjectName(e.currentTarget.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleCreateProject()}
                disabled={isCreating()}
              />
            </div>
            
            <div class="modal-action">
              <button 
                class="btn btn-ghost" 
                onClick={() => {
                  setIsModalOpen(false);
                  setNewProjectName('');
                }}
                disabled={isCreating()}
              >
                Cancel
              </button>
              <button 
                class="btn btn-primary" 
                onClick={handleCreateProject}
                disabled={!newProjectName().trim() || isCreating()}
              >
                <Show when={isCreating()}>
                  <span class="loading loading-spinner loading-xs mr-2"></span>
                </Show>
                Create
              </button>
            </div>
          </div>
          
          {/* Backdrop */}
          <div 
            class="modal-backdrop" 
            onClick={() => {
              if (!isCreating()) {
                setIsModalOpen(false);
                setNewProjectName('');
              }
            }}
          />
        </div>
      </Show>
    </>
  );
}

// Main App Content Component
function AppContent() {
  const { currentProject } = useProject();

  // Sheet state
  const [sheets, setSheets] = createSignal<Sheet[]>([]);
  const [currentSheet, setCurrentSheet] = createSignal<Sheet | null>(null);
  const [sheetTabs, setSheetTabs] = createSignal<SheetTab[]>([]);
  const [activeTabId, setActiveTabId] = createSignal<number | null>(null);
  const [columns, setColumns] = createSignal<SheetTabColumn[]>([]);
  const [rows, setRows] = createSignal<unknown[][]>([]);
  const [pagination, setPagination] = createSignal<SheetTabDataResult['pagination'] | null>(null);
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

  // Fetch sheets when project changes
  const loadSheets = async () => {
    const projectId = currentProject()?.id;
    if (!projectId) return;

    setIsLoadingSheets(true);
    setSheetError(null);
    try {
      const response = await fetch(`${API_BASE_URL}/api/sheets?project_id=${projectId}`);
      if (!response.ok) {
        throw new Error(`Failed to load sheets: ${response.status}`);
      }
      const data = await response.json() as ListSheetsResponse;
      setSheets(data.sheets);
      
      // Load "Nocodo Internal" sheet if available, otherwise first sheet
      const nocodoInternal = data.sheets.find(s => s.name === 'Nocodo Internal');
      if (nocodoInternal) {
        await loadSheet(nocodoInternal.id);
      } else if (data.sheets.length > 0) {
        await loadSheet(data.sheets[0].id);
      } else {
        setCurrentSheet(null);
        setSheetTabs([]);
        setColumns([]);
        setRows([]);
      }
    } catch (error) {
      console.error('Error loading sheets:', error);
      setSheetError(error instanceof Error ? error.message : 'Failed to load sheets');
    } finally {
      setIsLoadingSheets(false);
    }
  };

  // Load sheets when project changes
  onMount(() => {
    loadSheets();
  });

  // Reload sheets when current project changes
  const projectId = currentProject()?.id;
  if (projectId) {
    loadSheets();
  }

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

  // Load a tab's schema and data using the new dynamic endpoint
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

      // Load row data using the new dynamic endpoint
      const dataResponse = await fetch(
        `${API_BASE_URL}/api/sheets/data?sheet_tab_ids=${tabId}&limit=100`,
        { method: 'POST' }
      );
      if (!dataResponse.ok) {
        throw new Error(`Failed to load tab data: ${dataResponse.status}`);
      }
      const dataData = await dataResponse.json() as GetSheetDataResponse;

      // Use the first result (we only requested one tab)
      const result = dataData.results[0];
      if (result) {
        // Update columns from the result (includes column metadata from backend)
        setColumns(result.columns);
        setRows(result.rows);
        setPagination(result.pagination);
      } else {
        setRows([]);
        setPagination(null);
      }
    } catch (error) {
      console.error('Error loading tab:', error);
      setSheetError(error instanceof Error ? error.message : 'Failed to load tab');
    }
  };

  // Get display value for a cell from the row data (rowIndex is 0-based data row)
  // Uses positional array data from the new API (rows are unknown[][])
  const getCellDisplayValue = (colIndex: number, rowIndex: number): string => {
    const rowData = rows()[rowIndex]; // rowIndex 0 = displayed row 2

    if (!rowData || colIndex >= rowData.length) return '';

    const value = rowData[colIndex];

    if (value === null || value === undefined) return '';
    return String(value);
  };

  // Minimum number of columns to always show (A-Z = 26)
  const MIN_COLUMNS = 26;

  // Compute grid-template-columns based on column widths
  // Always ensure at least MIN_COLUMNS, using default width for empty columns
  const gridTemplateColumns = () => {
    const rowHeaderWidth = 56;
    const dataCols = columns();
    const colCount = Math.max(dataCols.length, MIN_COLUMNS);
    const colWidths = Array.from({ length: colCount }, (_, i) => {
      if (i < dataCols.length) {
        return `${dataCols[i].width || 120}px`;
      }
      return '120px'; // Default width for empty columns beyond data
    });
    return `${rowHeaderWidth}px ${colWidths.join(' ')}`;
  };

  // Get the raw cell value for the formula bar
  // Uses positional array data from the new API
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

    if (!rowData || colIndex < 0 || colIndex >= rowData.length) return '';

    const value = rowData[colIndex];

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
    
    const projectId = currentProject()?.id;
    if (!projectId) {
      setMessages(prev => [...prev, { 
        role: 'assistant', 
        content: 'Error: No project selected' 
      }]);
      return;
    }
    
    setMessages([...messages(), { role: 'user', content: message }]);
    setInputValue('');
    setIsLoading(true);
    
    try {
      const response = await fetch(`${API_BASE_URL}/api/agents/schema-designer/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          project_id: projectId,
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
        <ProjectSelector />
        <div class="menu-title">
          {currentSheet() ? currentSheet()!.name : 'Nocodo Sheets'}
        </div>
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

            <div class="grid-wrap" style={{ "grid-template-columns": gridTemplateColumns() }}>
              <div class="grid-corner" />
              {/* Column headers: Always A-Z (MIN_COLUMNS), even without data */}
              <For each={Array.from({ length: Math.max(columns().length, MIN_COLUMNS) }, (_, i) => i)}>
                {(i) => (
                  <div class="column-header">
                    {String.fromCharCode(65 + i)}
                  </div>
                )}
              </For>

              {/* Row 1: Column names from API */}
              <Show when={columns().length > 0}>
                {(() => {
                  const dataCols = columns();
                  const colCount = Math.max(dataCols.length, MIN_COLUMNS);
                  return (
                    <>
                      <div class="row-header">1</div>
                      <For each={Array.from({ length: colCount }, (_, i) => i)}>
                        {(colIndex) => {
                          const colLetter = String.fromCharCode(65 + colIndex);
                          const col = dataCols[colIndex];
                          return (
                            <div 
                              class={`cell${1 === selectedCell().row && colLetter === selectedCell().col ? ' cell-active' : ''}`}
                              onClick={() => handleCellClick(colLetter, 1)}
                            >
                              {col?.name ?? ''}
                            </div>
                          );
                        }}
                      </For>
                    </>
                  );
                })()}
              </Show>

              {/* Data rows from API (starting from row 2) */}
              <For each={rows()}>
                {(row, rowIndex) => {
                  // Capture values once
                  const dataCols = columns();
                  const colCount = Math.max(dataCols.length, MIN_COLUMNS);
                  const currentRow = rowIndex() + 2;
                  return (
                    <>
                      <div class="row-header">{currentRow}</div>
                      <For each={Array.from({ length: colCount }, (_, i) => i)}>
                        {(colIndex) => {
                          const colLetter = String.fromCharCode(65 + colIndex);
                          const hasData = colIndex < dataCols.length;
                          return (
                            <div 
                              class={`cell${currentRow === selectedCell().row && colLetter === selectedCell().col ? ' cell-active' : ''}`}
                              classList={{
                                'text-blue-600 underline cursor-pointer': hasData && isRelationColumn(colIndex)
                              }}
                              onClick={() => handleCellClick(colLetter, currentRow)}
                              title={hasData && isRelationColumn(colIndex) ? 'Click to view related record' : undefined}
                            >
                              {hasData ? getCellDisplayValue(colIndex, rowIndex()) : ''}
                            </div>
                          );
                        }}
                      </For>
                    </>
                  );
                }}
              </For>

              {/* Empty rows to fill up to 100 total rows (1 header + 99 data/empty rows) */}
              <Show when={columns().length === 0 || rows().length < 100}>
                {(() => {
                  // Capture values once to avoid re-evaluation issues
                  const dataCols = columns();
                  const colCount = Math.max(dataCols.length, MIN_COLUMNS);
                  const rowCount = rows().length;
                  const hasColumns = dataCols.length > 0;
                  const emptyRowCount = hasColumns ? Math.max(0, 99 - rowCount) : 100;
                  const startRowNum = hasColumns ? 2 + rowCount : 1;
                  
                  return (
                    <For each={Array.from({ length: emptyRowCount }, (_, i) => i)}>
                      {(rowOffset) => {
                        const rowNum = startRowNum + rowOffset;
                        return (
                          <>
                            <div class="row-header">{rowNum}</div>
                            <For each={Array.from({ length: colCount }, (_, i) => i)}>
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
                  );
                })()}
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

// Root App component with Provider
export default function App() {
  return (
    <ProjectProvider>
      <AppContent />
    </ProjectProvider>
  );
}
