import { For, Show, createEffect, createSignal } from 'solid-js';
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
    const session = chat.selectedSession();
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
