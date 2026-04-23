import { For, Show, createEffect, createSignal } from 'solid-js';
import { ProjectProvider, useProject } from './contexts/ProjectContext';
import type {
  Project,
  Schema,
  Table,
  Column,
  ListSchemasResponse,
  GetSchemaResponse,
  GetTableColumnsResponse,
  GetTableDataResponse,
  TableDataResult,
  SessionItem,
  ListSessionsResponse,
  SchemaDef,
} from './types/api';

const API_BASE_URL = '';  // Use relative URLs to leverage Vite proxy

type ChatRole = 'user' | 'assistant';
type UiMessage = { role: ChatRole; content: string };
type HistoryMessage = { id: number; role: string; content: string; created_at: number };

const DEFAULT_GREETING: UiMessage = {
  role: 'assistant',
  content: 'Hello! Tell me what you want to build and I\'ll design a schema for it.',
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

  const toggleDropdown = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDropdownOpen(!isDropdownOpen());
  };

  const closeDropdown = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDropdownOpen(false);
  };

  const selectProject = (project: Project) => {
    setCurrentProject(project);
    setIsDropdownOpen(false);
  };

  const openCreateModal = () => {
    setIsDropdownOpen(false);
    setIsModalOpen(true);
  };

  return (
    <>
      <div class="relative">
        <button class="btn btn-ghost btn-sm gap-2" onClick={toggleDropdown}>
          <Show when={isLoading()}>
            <span class="loading loading-spinner loading-xs"></span>
          </Show>
          <span class="font-semibold">{currentProject()?.name ?? 'Select Project'}</span>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>
        </button>

        <Show when={isDropdownOpen()}>
          <div class="absolute top-full left-0 z-[100] menu p-2 shadow bg-base-100 rounded-box w-56 mt-2 border border-base-300">
            <div class="px-3 py-2 text-xs font-semibold text-base-content/60 uppercase tracking-wider">
              Projects
            </div>
            <For each={projects()}>
              {(project) => (
                <button
                  class={`btn btn-ghost btn-sm justify-start ${project.id === currentProject()?.id ? 'btn-active' : ''}`}
                  onClick={() => selectProject(project)}
                >
                  <span class="truncate">{project.name}</span>
                </button>
              )}
            </For>

            <div class="divider my-1"></div>

            <button class="btn btn-ghost btn-sm justify-start text-primary" onClick={openCreateModal}>
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-1"><path d="M5 12h14"/><path d="M12 5v14"/></svg>
              Create Project
            </button>
          </div>
        </Show>

        <Show when={isDropdownOpen()}>
          <div class="fixed inset-0 z-[99]" onClick={closeDropdown} />
        </Show>
      </div>

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
                onClick={() => { setIsModalOpen(false); setNewProjectName(''); }}
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

          <div
            class="modal-backdrop"
            onClick={() => { if (!isCreating()) { setIsModalOpen(false); setNewProjectName(''); } }}
          />
        </div>
      </Show>
    </>
  );
}

// Main App Content Component
function AppContent() {
  const { currentProject } = useProject();

  // Schema/table state
  const [currentSchema, setCurrentSchema] = createSignal<Schema | null>(null);
  const [tables, setTables] = createSignal<Table[]>([]);
  const [activeTableId, setActiveTableId] = createSignal<number | null>(null);
  const [columns, setColumns] = createSignal<Column[]>([]);
  const [rows, setRows] = createSignal<unknown[][]>([]);
  const [pagination, setPagination] = createSignal<TableDataResult['pagination'] | null>(null);
  const [isLoadingSchemas, setIsLoadingSchemas] = createSignal(false);
  const [schemaError, setSchemaError] = createSignal<string | null>(null);

  // Cell/Grid state
  const [selectedCell, setSelectedCell] = createSignal({ col: 'A', row: 1 });
  const [cellFormulas, setCellFormulas] = createSignal<Record<string, string>>({});
  const [formulaValue, setFormulaValue] = createSignal('');

  // Sessions list state
  const [sessions, setSessions] = createSignal<SessionItem[]>([]);
  const [sessionsLoading, setSessionsLoading] = createSignal(false);

  // Drawer view: 'sessions' shows the list, 'chat' shows the chat UI
  const [drawerView, setDrawerView] = createSignal<'sessions' | 'chat'>('sessions');
  const [selectedSession, setSelectedSession] = createSignal<SessionItem | null>(null);

  // Chat state
  const [messages, setMessages] = createSignal<UiMessage[]>([DEFAULT_GREETING]);
  const [inputValue, setInputValue] = createSignal('');
  const [chatLoading, setChatLoading] = createSignal(false);

  // Schema preview state (populated when user clicks "Preview Schema")
  const [previewSchema, setPreviewSchema] = createSignal<SchemaDef | null>(null);
  const [previewTableIdx, setPreviewTableIdx] = createSignal(0);
  const [previewLoading, setPreviewLoading] = createSignal(false);

  // Reload schemas and sessions when project changes
  createEffect(() => {
    const project = currentProject();
    if (!project) return;
    loadSchemas(project.id);
    loadSessions(project.id);
  });

  const loadSchemas = async (projectId: number) => {
    setIsLoadingSchemas(true);
    setSchemaError(null);
    try {
      const response = await fetch(`${API_BASE_URL}/api/schemas?project_id=${projectId}`);
      if (!response.ok) throw new Error(`Failed to load schemas: ${response.status}`);
      const data = await response.json() as ListSchemasResponse;

      // Load "Nocodo Internal" schema if available, otherwise first schema
      const nocodoInternal = data.schemas.find(s => s.name === 'Nocodo Internal');
      if (nocodoInternal) {
        await loadSchema(nocodoInternal.id);
      } else if (data.schemas.length > 0) {
        await loadSchema(data.schemas[0].id);
      } else {
        setCurrentSchema(null);
        setTables([]);
        setColumns([]);
        setRows([]);
      }
    } catch (error) {
      console.error('Error loading schemas:', error);
      setSchemaError(error instanceof Error ? error.message : 'Failed to load schemas');
    } finally {
      setIsLoadingSchemas(false);
    }
  };

  const loadSchema = async (schemaId: number) => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/schemas/${schemaId}`);
      if (!response.ok) throw new Error(`Failed to load schema: ${response.status}`);
      const data = await response.json() as GetSchemaResponse;
      setCurrentSchema(data.schema);
      setTables(data.tables);

      if (data.tables.length > 0) {
        await loadTable(data.tables[0].id);
      }
    } catch (error) {
      console.error('Error loading schema:', error);
      setSchemaError(error instanceof Error ? error.message : 'Failed to load schema');
    }
  };

  const loadTable = async (tableId: number) => {
    setActiveTableId(tableId);
    try {
      const colResponse = await fetch(`${API_BASE_URL}/api/tables/${tableId}/columns`);
      if (!colResponse.ok) throw new Error(`Failed to load columns: ${colResponse.status}`);
      const colData = await colResponse.json() as GetTableColumnsResponse;
      setColumns(colData.columns);

      const dataResponse = await fetch(
        `${API_BASE_URL}/api/tables/data?table_ids=${tableId}&limit=100`,
        { method: 'POST' }
      );
      if (!dataResponse.ok) throw new Error(`Failed to load table data: ${dataResponse.status}`);
      const dataData = await dataResponse.json() as GetTableDataResponse;

      const result = dataData.results[0];
      if (result) {
        setColumns(result.columns);
        setRows(result.rows);
        setPagination(result.pagination);
      } else {
        setRows([]);
        setPagination(null);
      }
    } catch (error) {
      console.error('Error loading table:', error);
      setSchemaError(error instanceof Error ? error.message : 'Failed to load table');
    }
  };

  const loadSessions = async (projectId: number) => {
    setSessionsLoading(true);
    try {
      const response = await fetch(`${API_BASE_URL}/api/agents/sessions?project_id=${projectId}`);
      if (!response.ok) throw new Error(`Failed to load sessions: ${response.status}`);
      const data = await response.json() as ListSessionsResponse;
      setSessions(data.sessions);
      // No sessions yet — go straight to the chat UI
      if (data.sessions.length === 0) {
        setDrawerView('chat');
        setSelectedSession(null);
      }
    } catch (error) {
      console.error('Error loading sessions:', error);
    } finally {
      setSessionsLoading(false);
    }
  };

  const formatSessionDate = (unixTs: number) =>
    new Date(unixTs * 1000).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });

  const agentTypeLabel = (type: string) =>
    type.split('_').map(w => w[0].toUpperCase() + w.slice(1)).join(' ');

  const selectSession = async (session: SessionItem) => {
    setSelectedSession(session);
    setDrawerView('chat');
    setMessages([DEFAULT_GREETING]);
    setChatLoading(true);
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/agents/schema-designer/sessions/${session.id}/messages`
      );
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json() as { messages?: HistoryMessage[] };
      const history = (data.messages ?? [])
        .filter(m => m.role === 'user' || m.role === 'assistant')
        .map(m => ({ role: m.role as ChatRole, content: m.content }));
      if (history.length > 0) setMessages(history);
    } catch (error) {
      console.error('Error loading session history:', error);
    } finally {
      setChatLoading(false);
    }
  };

  const backToSessions = () => {
    setDrawerView('sessions');
    setSelectedSession(null);
    setMessages([DEFAULT_GREETING]);
    setInputValue('');
    // Refresh session list (may have grown since the drawer was opened)
    const p = currentProject();
    if (p) loadSessions(p.id);
  };

  const pollForResponse = async (messageId: number, sessionId: number) => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/agents/schema-designer/messages/${messageId}/response`
      );
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json();
      if (data.response?.type === 'pending') {
        setTimeout(() => pollForResponse(messageId, sessionId), 500);
        return;
      }
      // Reload full history so tool messages are filtered correctly
      const histRes = await fetch(
        `${API_BASE_URL}/api/agents/schema-designer/sessions/${sessionId}/messages`
      );
      if (histRes.ok) {
        const histData = await histRes.json() as { messages?: HistoryMessage[] };
        const history = (histData.messages ?? [])
          .filter(m => m.role === 'user' || m.role === 'assistant')
          .map(m => ({ role: m.role as ChatRole, content: m.content }));
        if (history.length > 0) setMessages(history);
      }
    } catch (error) {
      console.error('Error polling response:', error);
      setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${error instanceof Error ? error.message : 'Unknown error'}` }]);
    } finally {
      setChatLoading(false);
    }
  };

  const handleSend = async () => {
    const message = inputValue().trim();
    const session = selectedSession();
    const projectId = currentProject()?.id;
    if (!message || !projectId) return;

    setMessages(prev => [...prev, { role: 'user', content: message }]);
    setInputValue('');
    setChatLoading(true);

    try {
      const body: Record<string, unknown> = { project_id: projectId, message };
      if (session) body.session_id = session.id;

      const response = await fetch(`${API_BASE_URL}/api/agents/schema-designer/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json() as { session_id: number; message_id: number };

      // Backend created a new session — capture it so the rest of the chat works
      if (!session && data.session_id) {
        setSelectedSession({
          id: data.session_id,
          project_id: projectId,
          agent_type: 'schema_designer',
          created_at: Math.floor(Date.now() / 1000),
        });
      }

      if (data.message_id) {
        pollForResponse(data.message_id, data.session_id ?? session!.id);
      } else {
        setChatLoading(false);
      }
    } catch (error) {
      console.error('Error sending message:', error);
      setChatLoading(false);
      setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${error instanceof Error ? error.message : 'Failed to send'}` }]);
    }
  };

  const fetchPreviewSchema = async () => {
    const session = selectedSession();
    if (!session) return;
    setPreviewLoading(true);
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/agents/schema-designer/sessions/${session.id}/schema`
      );
      if (!response.ok) {
        const err = await response.json() as { error?: string };
        throw new Error(err.error ?? `HTTP ${response.status}`);
      }
      const data = await response.json() as { schema: SchemaDef; version: number };
      setPreviewSchema(data.schema);
      setPreviewTableIdx(0);
    } catch (error) {
      console.error('Error fetching preview schema:', error);
      setMessages(prev => [...prev, {
        role: 'assistant',
        content: `Could not load schema: ${error instanceof Error ? error.message : 'Unknown error'}`,
      }]);
    } finally {
      setPreviewLoading(false);
    }
  };

  // Minimum number of columns to always show (A-Z = 26)
  const MIN_COLUMNS = 26;

  // When a preview schema is active, use its columns; otherwise use live DB columns.
  const displayColumns = (): { name: string }[] => {
    const ps = previewSchema();
    if (ps) return ps.tables[previewTableIdx()]?.columns ?? [];
    return columns();
  };

  const gridTemplateColumns = () => {
    const rowHeaderWidth = 56;
    const colCount = Math.max(displayColumns().length, MIN_COLUMNS);
    const colWidths = Array.from({ length: colCount }, () => '120px');
    return `${rowHeaderWidth}px ${colWidths.join(' ')}`;
  };

  const getCellDisplayValue = (colIndex: number, rowIndex: number): string => {
    const rowData = rows()[rowIndex];
    if (!rowData || colIndex >= rowData.length) return '';
    const value = rowData[colIndex];
    if (value === null || value === undefined) return '';
    return String(value);
  };

  const getCellKey = (col: string, row: number) => `${col}${row}`;

  const getCellValue = (col: string, row: number): string => {
    const colIndex = col.charCodeAt(0) - 65;
    const cols = columns();

    const key = getCellKey(col, row);
    const formula = cellFormulas()[key];
    if (formula !== undefined && formula !== '') return formula;

    if (row === 1 && colIndex >= 0 && colIndex < cols.length) {
      return cols[colIndex].name;
    }

    const rowIndex = row - 2;
    const rowData = rows()[rowIndex];
    if (!rowData || colIndex < 0 || colIndex >= rowData.length) return '';
    const value = rowData[colIndex];
    if (value === null || value === undefined) return '';
    return String(value);
  };

  const handleCellClick = (col: string, row: number) => {
    setSelectedCell({ col, row });
    setFormulaValue(getCellValue(col, row));
  };

  const handleFormulaChange = (value: string) => {
    setFormulaValue(value);
    const key = getCellKey(selectedCell().col, selectedCell().row);
    setCellFormulas(prev => ({ ...prev, [key]: value }));
  };

  return (
    <main class="sheet-app">
      <header class="menu-strip">
        <ProjectSelector />
        <div class="menu-title">
          {currentSchema() ? currentSchema()!.name : 'Nocodo Sheets'}
        </div>
        <div class="menu-status">
          <Show when={isLoadingSchemas()}>
            <span class="loading loading-spinner loading-xs mr-2"></span>
          </Show>
          {schemaError() ? 'Error' : 'Synced'}
        </div>
      </header>

      <div class="drawer">
        <input id="chat-drawer" type="checkbox" class="drawer-toggle" />

        <div class="drawer-content flex flex-col">
          <section class="sheet-main">
            <div class="formula-strip">
              <label for="chat-drawer" class="btn btn-success btn-sm">
                AI Assistant
              </label>
              <div class="name-box">{selectedCell().col}{selectedCell().row}</div>
              <label class="formula-label" for="formula-input">fx</label>
              <input
                id="formula-input"
                class="formula-input"
                value={formulaValue()}
                onInput={(e) => handleFormulaChange(e.currentTarget.value)}
                aria-label="Formula bar"
              />
            </div>

            <Show when={schemaError()}>
              <div class="alert alert-error m-4">
                <span>{schemaError()}</span>
                <button class="btn btn-sm btn-ghost" onClick={() => { const p = currentProject(); if (p) loadSchemas(p.id); }}>Retry</button>
              </div>
            </Show>

            {/* Preview mode banner */}
            <Show when={previewSchema() !== null}>
              <div class="flex items-center gap-2 px-3 py-1.5 bg-success/10 border-b border-success/30 text-sm">
                <span class="text-success font-medium">Previewing: {previewSchema()!.name}</span>
                <button class="btn btn-ghost btn-xs ml-auto" onClick={() => setPreviewSchema(null)}>
                  ✕ Exit preview
                </button>
              </div>
            </Show>

            <div class="grid-wrap" style={{ "grid-template-columns": gridTemplateColumns() }}>
              <div class="grid-corner" />
              <For each={Array.from({ length: Math.max(displayColumns().length, MIN_COLUMNS) }, (_, i) => i)}>
                {(i) => <div class="column-header">{String.fromCharCode(65 + i)}</div>}
              </For>

              {/* Row 1: column names */}
              <Show when={displayColumns().length > 0}>
                {(() => {
                  const dataCols = displayColumns();
                  const colCount = Math.max(dataCols.length, MIN_COLUMNS);
                  return (
                    <>
                      <div class="row-header">1</div>
                      <For each={Array.from({ length: colCount }, (_, i) => i)}>
                        {(colIndex) => {
                          const colLetter = String.fromCharCode(65 + colIndex);
                          return (
                            <div
                              class={`cell${1 === selectedCell().row && colLetter === selectedCell().col ? ' cell-active' : ''}`}
                              onClick={() => handleCellClick(colLetter, 1)}
                            >
                              {dataCols[colIndex]?.name ?? ''}
                            </div>
                          );
                        }}
                      </For>
                    </>
                  );
                })()}
              </Show>

              {/* Data rows (hidden in preview mode) */}
              <Show when={previewSchema() === null}>
                <For each={rows()}>
                  {(_, rowIndex) => {
                    const dataCols = displayColumns();
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
                                onClick={() => handleCellClick(colLetter, currentRow)}
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
              </Show>

              {/* Empty rows to fill to 100 */}
              <Show when={displayColumns().length === 0 || (previewSchema() === null ? rows().length < 100 : true)}>
                {(() => {
                  const dataCols = displayColumns();
                  const colCount = Math.max(dataCols.length, MIN_COLUMNS);
                  const rowCount = previewSchema() ? 0 : rows().length;
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
            <button class="sheet-add" aria-label="Add sheet">+</button>
            <div class="sheet-list">
              {/* Preview mode: show schema tables as tabs */}
              <Show when={previewSchema() !== null}>
                <For each={previewSchema()!.tables}>
                  {(table, i) => (
                    <button
                      class={`sheet-tab${i() === previewTableIdx() ? ' sheet-tab-active' : ''}`}
                      onClick={() => setPreviewTableIdx(i())}
                    >
                      {table.name}
                    </button>
                  )}
                </For>
              </Show>

              {/* Normal mode: show live DB tables */}
              <Show when={previewSchema() === null}>
                <Show when={tables().length === 0}>
                  <span class="text-sm text-gray-500 px-2">No tables available</span>
                </Show>
                <For each={tables()}>
                  {(table) => (
                    <button
                      class={`sheet-tab${table.id === activeTableId() ? ' sheet-tab-active' : ''}`}
                      onClick={() => loadTable(table.id)}
                    >
                      {table.name}
                    </button>
                  )}
                </For>
              </Show>
            </div>
          </footer>
        </div>

        <div class="drawer-side z-50">
          <label for="chat-drawer" aria-label="close sidebar" class="drawer-overlay"></label>
          <div class="chat-sidebar">

            {/* Sessions list view */}
            <Show when={drawerView() === 'sessions'}>
              <div class="chat-panel-header">
                <h3 class="text-sm font-semibold">AI Assistant</h3>
                <label for="chat-drawer" class="btn btn-ghost btn-sm btn-square">✕</label>
              </div>

              <div class="flex-1 overflow-y-auto">
                <Show when={sessionsLoading()}>
                  <div class="flex justify-center p-6">
                    <span class="loading loading-spinner loading-sm"></span>
                  </div>
                </Show>
                <Show when={!sessionsLoading() && sessions().length === 0}>
                  <div class="p-6 text-sm text-base-content/50 text-center">No sessions yet</div>
                </Show>
                <Show when={!sessionsLoading() && sessions().length > 0}>
                  <ul class="list bg-base-100">
                    <For each={sessions()}>
                      {(session) => (
                        <li
                          class="list-row items-center cursor-pointer hover:bg-base-200 transition-colors"
                          onClick={() => selectSession(session)}
                        >
                          <div>
                            <div class="badge badge-primary badge-soft badge-sm">
                              {agentTypeLabel(session.agent_type)}
                            </div>
                          </div>
                          <div class="list-col-grow">
                            <p class="text-sm font-medium">Session {session.id}</p>
                            <p class="text-xs text-base-content/50">{formatSessionDate(session.created_at)}</p>
                          </div>
                          <div class="text-base-content/30">›</div>
                        </li>
                      )}
                    </For>
                  </ul>
                </Show>
              </div>
            </Show>

            {/* Chat view */}
            <Show when={drawerView() === 'chat'}>
              <div class="chat-panel-header">
                <Show when={sessions().length > 0}>
                  <button class="btn btn-ghost btn-sm btn-square" onClick={backToSessions}>‹</button>
                </Show>
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-semibold truncate">
                    {selectedSession() ? agentTypeLabel(selectedSession()!.agent_type) : 'Schema Designer'}
                  </p>
                  <p class="text-xs text-base-content/50">
                    {selectedSession() ? `Session ${selectedSession()!.id}` : 'New session'}
                  </p>
                </div>
                <label for="chat-drawer" class="btn btn-ghost btn-sm btn-square">✕</label>
              </div>

              <div class="chat-messages">
                <Show when={chatLoading() && messages().length <= 1}>
                  <div class="flex justify-center p-4">
                    <span class="loading loading-spinner loading-sm"></span>
                  </div>
                </Show>
                <For each={messages()}>
                  {(msg) => (
                    <div class={`chat ${msg.role === 'user' ? 'chat-end' : 'chat-start'}`}>
                      <div class={`chat-bubble whitespace-pre-wrap ${msg.role === 'user' ? 'chat-bubble-primary' : ''}`}>
                        {msg.content}
                      </div>
                    </div>
                  )}
                </For>
                <Show when={chatLoading() && messages().length > 1}>
                  <div class="chat chat-start">
                    <div class="chat-bubble">
                      <span class="loading loading-dots loading-xs"></span>
                    </div>
                  </div>
                </Show>
              </div>

              <div class="border-t border-base-300 px-4 py-3 bg-base-200">
                <button
                  class="btn btn-success btn-sm w-full"
                  onClick={fetchPreviewSchema}
                  disabled={previewLoading()}
                >
                  <Show when={previewLoading()}>
                    <span class="loading loading-spinner loading-xs mr-1"></span>
                  </Show>
                  Preview Schema
                </button>
              </div>

              <div class="chat-input-area">
                <div class="chat-input-row">
                  <input
                    type="text"
                    placeholder="Describe a schema..."
                    class="input input-bordered input-sm flex-1"
                    value={inputValue()}
                    onInput={(e) => setInputValue(e.currentTarget.value)}
                    onKeyDown={(e) => e.key === 'Enter' && !chatLoading() && handleSend()}
                    disabled={chatLoading()}
                  />
                  <button
                    class="btn btn-success btn-sm"
                    onClick={handleSend}
                    disabled={chatLoading() || !inputValue().trim()}
                  >
                    Send
                  </button>
                </div>
              </div>
            </Show>

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
