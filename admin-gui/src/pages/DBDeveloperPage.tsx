import { For, Show, createEffect, createSignal } from 'solid-js';
import { useParams } from '@solidjs/router';
import { useProject } from '../contexts/ProjectContext';
import ProjectSelector from '../components/ProjectSelector';
import type {
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
} from '../types/api';

const API_BASE_URL = '';

type ChatRole = 'user' | 'assistant';
type UiMessage = { role: ChatRole; content: string; schema_version?: number };
type HistoryMessage = { id: number; role: string; content: string; created_at: number; schema_version?: number };

const DEFAULT_GREETING: UiMessage = {
  role: 'assistant',
  content: "Hello! Tell me what you want to build and I'll design a schema for it.",
};

const MIN_COLUMNS = 26;

export default function DBDeveloperPage() {
  const params = useParams<{ projectId: string }>();
  const { projects, currentProject, setCurrentProject } = useProject();

  const parsedProjectId = () => {
    const n = Number.parseInt(params.projectId, 10);
    return Number.isFinite(n) ? n : null;
  };

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

  // Sessions state
  const [sessions, setSessions] = createSignal<SessionItem[]>([]);
  const [sessionsLoading, setSessionsLoading] = createSignal(false);
  const [selectedAgent] = createSignal<string>('schema_designer');
  const [selectedSession, setSelectedSession] = createSignal<SessionItem | null>(null);

  // Chat state
  const [messages, setMessages] = createSignal<UiMessage[]>([DEFAULT_GREETING]);
  const [inputValue, setInputValue] = createSignal('');
  const [chatLoading, setChatLoading] = createSignal(false);

  // Schema preview state
  const [previewSchema, setPreviewSchema] = createSignal<SchemaDef | null>(null);
  const [previewTableIdx, setPreviewTableIdx] = createSignal(0);
  const [previewLoading, setPreviewLoading] = createSignal(false);

  // Sync currentProject from URL param
  createEffect(() => {
    const targetId = parsedProjectId();
    if (!targetId) return;
    const project = projects().find((p) => p.id === targetId) ?? null;
    if (project && currentProject()?.id !== targetId) {
      setCurrentProject(project);
    }
  });

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
      if (data.tables.length > 0) await loadTable(data.tables[0].id);
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

  const loadSessions = async (projectId: number, agentType?: string) => {
    setSessionsLoading(true);
    try {
      const type = agentType ?? selectedAgent();
      const url = `${API_BASE_URL}/api/agents/sessions?project_id=${projectId}&agent_type=${type}`;
      const response = await fetch(url);
      if (!response.ok) throw new Error(`Failed to load sessions: ${response.status}`);
      const data = await response.json() as ListSessionsResponse;
      setSessions(data.sessions);
      if (data.sessions.length > 0) {
        const latest = data.sessions.sort((a, b) => b.created_at - a.created_at)[0];
        await selectSession(latest);
      } else {
        setSelectedSession(null);
      }
    } catch (error) {
      console.error('Error loading sessions:', error);
    } finally {
      setSessionsLoading(false);
    }
  };

  const selectSession = async (session: SessionItem) => {
    setSelectedSession(session);
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
        .map(m => ({ role: m.role as ChatRole, content: m.content, schema_version: m.schema_version }));
      if (history.length > 0) setMessages(history);
    } catch (error) {
      console.error('Error loading session history:', error);
    } finally {
      setChatLoading(false);
    }
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
      const histRes = await fetch(
        `${API_BASE_URL}/api/agents/schema-designer/sessions/${sessionId}/messages`
      );
      if (histRes.ok) {
        const histData = await histRes.json() as { messages?: HistoryMessage[] };
        const history = (histData.messages ?? [])
          .filter(m => m.role === 'user' || m.role === 'assistant')
          .map(m => ({ role: m.role as ChatRole, content: m.content, schema_version: m.schema_version }));
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

  const fetchPreviewSchema = async (version?: number) => {
    const session = selectedSession();
    if (!session) return;
    setPreviewLoading(true);
    try {
      const url = version
        ? `${API_BASE_URL}/api/agents/schema-designer/sessions/${session.id}/schema?version=${version}`
        : `${API_BASE_URL}/api/agents/schema-designer/sessions/${session.id}/schema`;
      const response = await fetch(url);
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

  const displayColumns = (): { name: string; label?: string | null }[] => {
    const ps = previewSchema();
    if (ps) return ps.tables[previewTableIdx()]?.columns ?? [];
    return columns();
  };

  const displayColumnHeader = (col: { name: string; label?: string | null }): string =>
    col.label?.trim() || col.name;

  const gridTemplateColumns = () => {
    const rowHeaderWidth = 56;
    const colCount = Math.max(displayColumns().length, MIN_COLUMNS);
    return `${rowHeaderWidth}px ${Array.from({ length: colCount }, () => '120px').join(' ')}`;
  };

  const getCellDisplayValue = (colIndex: number, rowIndex: number): string => {
    const rowData = rows()[rowIndex];
    if (!rowData || colIndex >= rowData.length) return '';
    const value = rowData[colIndex];
    return value === null || value === undefined ? '' : String(value);
  };

  const getCellKey = (col: string, row: number) => `${col}${row}`;

  const getCellValue = (col: string, row: number): string => {
    const colIndex = col.charCodeAt(0) - 65;
    const cols = columns();
    const key = getCellKey(col, row);
    const formula = cellFormulas()[key];
    if (formula !== undefined && formula !== '') return formula;
    if (row === 1 && colIndex >= 0 && colIndex < cols.length) return cols[colIndex].name;
    const rowIndex = row - 2;
    const rowData = rows()[rowIndex];
    if (!rowData || colIndex < 0 || colIndex >= rowData.length) return '';
    const value = rowData[colIndex];
    return value === null || value === undefined ? '' : String(value);
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

  // suppress unused warning — sessions list used for future session switching UI
  void sessions();
  void sessionsLoading();

  return (
    <main class="sheet-app">
      <header class="menu-strip">
        <ProjectSelector />
        <div class="menu-title">
          {currentSchema() ? currentSchema()!.name : 'Nocodo Sheets'}
        </div>
        <div class="menu-status">
          <Show when={isLoadingSchemas()}>
            <span class="loading loading-spinner loading-xs mr-2" />
          </Show>
          {schemaError() ? 'Error' : 'Synced'}
        </div>
      </header>

      <div class="drawer">
        <input id="chat-drawer" type="checkbox" class="drawer-toggle" />

        <div class="drawer-content flex flex-col">
          <section class="sheet-main">
            <div class="formula-strip">
              <label for="chat-drawer" class="btn btn-success btn-sm">Dev Team</label>
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

            <Show when={previewSchema() !== null}>
              <div class="flex items-center gap-2 px-3 py-1.5 bg-success/10 border-b border-success/30 text-sm">
                <span class="text-success font-medium">Previewing: {previewSchema()!.name}</span>
                <button class="btn btn-ghost btn-xs ml-auto" onClick={() => setPreviewSchema(null)}>✕ Exit preview</button>
              </div>
            </Show>

            <div class="grid-wrap" style={{ "grid-template-columns": gridTemplateColumns() }}>
              <div class="grid-corner" />
              <For each={Array.from({ length: Math.max(displayColumns().length, MIN_COLUMNS) }, (_, i) => i)}>
                {(i) => <div class="column-header">{String.fromCharCode(65 + i)}</div>}
              </For>

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
                              {dataCols[colIndex] ? displayColumnHeader(dataCols[colIndex]) : ''}
                            </div>
                          );
                        }}
                      </For>
                    </>
                  );
                })()}
              </Show>

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
                            return (
                              <div
                                class={`cell${currentRow === selectedCell().row && colLetter === selectedCell().col ? ' cell-active' : ''}`}
                                onClick={() => handleCellClick(colLetter, currentRow)}
                              >
                                {colIndex < dataCols.length ? getCellDisplayValue(colIndex, rowIndex()) : ''}
                              </div>
                            );
                          }}
                        </For>
                      </>
                    );
                  }}
                </For>
              </Show>

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
            </div>
          </section>

          <footer class="sheets-strip">
            <button class="sheet-add" aria-label="Add sheet">+</button>
            <div class="sheet-list">
              <Show when={previewSchema() !== null}>
                <For each={previewSchema()!.tables}>
                  {(table, i) => (
                    <button
                      class={`sheet-tab${i() === previewTableIdx() ? ' sheet-tab-active' : ''}`}
                      onClick={() => setPreviewTableIdx(i())}
                    >
                      {table.label?.trim() || table.name}
                    </button>
                  )}
                </For>
              </Show>
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
          <label for="chat-drawer" aria-label="close sidebar" class="drawer-overlay" />
          <div class="chat-sidebar">
            <div class="agent-list">
              <div class="chat-panel-header">
                <h3 class="text-sm font-semibold">Dev Team</h3>
                <label for="chat-drawer" class="btn btn-ghost btn-sm btn-square">✕</label>
              </div>
              <div class="flex-1 overflow-y-auto">
                <ul class="list bg-base-100">
                  <li class="list-row items-center cursor-default bg-base-200">
                    <div class="avatar placeholder">
                      <div class="bg-neutral text-neutral-content w-10 h-10 rounded-full flex items-center justify-center">
                        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/><path d="M3 12c0 1.66 4 3 9 3s9-1.34 9-3"/></svg>
                      </div>
                    </div>
                    <div class="list-col-grow">
                      <p class="text-sm font-medium">Database Dev</p>
                      <p class="text-xs text-base-content/50">Online</p>
                    </div>
                  </li>
                </ul>
              </div>
            </div>

            <div class="chat-panel">
              <div class="chat-panel-header">
                <p class="text-sm font-semibold flex-1 truncate">Database Dev</p>
                <label for="chat-drawer" class="btn btn-ghost btn-sm btn-square">✕</label>
              </div>

              <div class="chat-messages">
                <Show when={chatLoading() && messages().length <= 1}>
                  <div class="flex justify-center p-4">
                    <span class="loading loading-spinner loading-sm" />
                  </div>
                </Show>
                <For each={messages()}>
                  {(msg) => (
                    <div class={`chat ${msg.role === 'user' ? 'chat-end' : 'chat-start'}`}>
                      <div class={`chat-bubble whitespace-pre-wrap ${msg.role === 'user' ? 'chat-bubble-primary' : ''}`}>
                        <Show when={msg.schema_version != null} fallback={msg.content}>
                          <div class="flex flex-col gap-2">
                            <span>{msg.content || 'Designed a schema'}</span>
                            <button
                              class="btn btn-xs btn-outline self-start"
                              onClick={() => fetchPreviewSchema(msg.schema_version)}
                              disabled={previewLoading()}
                            >
                              Preview Schema, V{msg.schema_version}
                            </button>
                          </div>
                        </Show>
                      </div>
                    </div>
                  )}
                </For>
                <Show when={chatLoading() && messages().length > 1}>
                  <div class="chat chat-start">
                    <div class="chat-bubble">
                      <span class="loading loading-dots loading-xs" />
                    </div>
                  </div>
                </Show>
              </div>

              <div class="border-t border-base-300 px-4 py-3 bg-base-200">
                <button
                  class="btn btn-success btn-sm w-full"
                  onClick={() => fetchPreviewSchema()}
                  disabled={previewLoading()}
                >
                  <Show when={previewLoading()}>
                    <span class="loading loading-spinner loading-xs mr-1" />
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
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
