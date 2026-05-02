import { For, Show, createEffect, createMemo, createSignal } from 'solid-js';
import { useParams } from '@solidjs/router';
import { useProject } from '../contexts/ProjectContext';
import { ChatProvider, useChat } from '../contexts/ChatContext';
import ChatDrawer from '../components/ChatDrawer';
import type {
  Schema,
  Table,
  Column,
  ListSchemasResponse,
  GetSchemaResponse,
  GetTableColumnsResponse,
  GetTableDataResponse,
  TableDataResult,
  SchemaDef,
} from '../types/api';

const API_BASE_URL = '';

const MIN_COLUMNS = 26;

export default function DBDeveloperPage() {
  const { currentProject } = useProject();

  return (
    <ChatProvider agentType="schema_designer" projectId={() => currentProject()?.id}>
      <DBDeveloperContent />
    </ChatProvider>
  );
}

function DBDeveloperContent() {
  const params = useParams<{ projectId: string }>();
  const { projects, currentProject, setCurrentProject } = useProject();
  const chat = useChat();

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

  const fetchPreviewSchema = async (version?: number) => {
    const task = chat.selectedTask();
    if (!task) return;
    setPreviewLoading(true);
    try {
      const url = version
        ? `${API_BASE_URL}/api/agents/schema-designer/tasks/${task.id}/schema?version=${version}`
        : `${API_BASE_URL}/api/agents/schema-designer/tasks/${task.id}/schema`;
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
    const key = getCellKey(col, row);
    const formula = cellFormulas()[key];
    if (formula !== undefined && formula !== '') return formula;
    const rowData = rows()[row - 1];
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

  const columnHeaders = createMemo(() => {
    const cols = displayColumns();
    return Array.from({ length: Math.max(cols.length, MIN_COLUMNS) }, (_, i) => ({
      i,
      col: cols[i] ?? null,
    }));
  });

  const placeholder = () =>
    chat.messages().length > 1
      ? 'I am a database developer: Would you like some changes in the database design?'
      : 'I am a database developer: Tell me the workflow you have in mind, I will help you design the database.';

  return (
    <main class="sheet-app">
      <ChatDrawer
        agentName="Database Dev"
        placeholder={placeholder}
        renderMessage={(msg) => (
          <div class={`chat ${msg.role === 'user' ? 'chat-end' : 'chat-start'}`}>
            <div class={`chat-bubble whitespace-pre-wrap ${msg.role === 'user' ? 'chat-bubble-primary' : 'bg-transparent'}`}>
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
      >
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
            <For each={columnHeaders()}>
              {(item) => item.col ? (
                <div class="column-header column-header-field">
                  <span class="truncate">{displayColumnHeader(item.col)}</span>
                  <details class="dropdown dropdown-end" onClick={(e) => e.stopPropagation()}>
                    <summary class="btn btn-ghost btn-xs p-0 h-5 min-h-5 w-5 opacity-40 hover:opacity-100 flex items-center justify-center">
                      ▾
                    </summary>
                    <ul class="dropdown-content menu bg-base-100 rounded-box shadow-md w-44 p-1 text-sm">
                      <li><a>Edit column</a></li>
                      <li><a>← Move left</a></li>
                      <li><a>→ Move right</a></li>
                      <li class="border-t border-base-200 my-0.5 pointer-events-none" />
                      <li><a>Hide field</a></li>
                      <li><a class="text-error">Remove field</a></li>
                    </ul>
                  </details>
                </div>
              ) : (
                <div class="column-header">{String.fromCharCode(65 + item.i)}</div>
              )}
            </For>

            <Show when={previewSchema() === null}>
              <For each={rows()}>
                {(_, rowIndex) => {
                  const dataCols = displayColumns();
                  const colCount = Math.max(dataCols.length, MIN_COLUMNS);
                  const currentRow = rowIndex() + 1;
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
              const startRowNum = hasColumns ? 1 + rowCount : 1;
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
          <button class="btn btn-ghost btn-xs btn-square" aria-label="Add sheet">+</button>
          <div class="tabs tabs-border tabs-sm">
            <Show when={previewSchema() !== null}>
              <For each={previewSchema()!.tables}>
                {(table, i) => (
                  <button
                    class={`tab${i() === previewTableIdx() ? ' tab-active' : ''}`}
                    onClick={() => setPreviewTableIdx(i())}
                  >
                    {table.label?.trim() || table.name}
                  </button>
                )}
              </For>
            </Show>
            <Show when={previewSchema() === null}>
              <Show when={tables().length === 0}>
                <span class="tab text-base-content/50 cursor-default">No tables available</span>
              </Show>
              <For each={tables()}>
                {(table) => (
                  <button
                    class={`tab${table.id === activeTableId() ? ' tab-active' : ''}`}
                    onClick={() => loadTable(table.id)}
                  >
                    {table.name}
                  </button>
                )}
              </For>
            </Show>
          </div>
        </footer>
      </ChatDrawer>
    </main>
  );
}
