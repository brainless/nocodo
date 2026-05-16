import { For, Show, createEffect, createMemo, createSignal } from 'solid-js';
import { useParams } from '@solidjs/router';
import { useProject } from '../contexts/ProjectContext';
import ProjectTopNav from '../components/ProjectTopNav';
import type {
  Schema,
  Table,
  Column,
  ListSchemasResponse,
  GetSchemaResponse,
  GetTableColumnsResponse,
  GetTableDataResponse,
  TableDataResult,
} from '../types/api';

const API_BASE_URL = '';

const MIN_COLUMNS = 26;

export default function DatabasePage() {
  const params = useParams<{ projectId: string }>();
  const { projects, currentProject, setCurrentProject } = useProject();

  const parsedProjectId = () => {
    const n = Number.parseInt(params.projectId, 10);
    return Number.isFinite(n) ? n : null;
  };

  const [currentSchema, setCurrentSchema] = createSignal<Schema | null>(null);
  const [tables, setTables] = createSignal<Table[]>([]);
  const [activeTableId, setActiveTableId] = createSignal<number | null>(null);
  const [columns, setColumns] = createSignal<Column[]>([]);
  const [rows, setRows] = createSignal<unknown[][]>([]);
  const [pagination, setPagination] = createSignal<TableDataResult['pagination'] | null>(null);
  const [isLoadingSchemas, setIsLoadingSchemas] = createSignal(false);
  const [schemaError, setSchemaError] = createSignal<string | null>(null);
  const [selectedCell, setSelectedCell] = createSignal({ col: 'A', row: 1 });

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

  const gridTemplateColumns = () => {
    const rowHeaderWidth = 56;
    const colCount = Math.max(columns().length, MIN_COLUMNS);
    return `${rowHeaderWidth}px ${Array.from({ length: colCount }, () => '120px').join(' ')}`;
  };

  const getCellDisplayValue = (colIndex: number, rowIndex: number): string => {
    const rowData = rows()[rowIndex];
    if (!rowData || colIndex >= rowData.length) return '';
    const value = rowData[colIndex];
    return value === null || value === undefined ? '' : String(value);
  };

  const handleCellClick = (col: string, row: number) => {
    setSelectedCell({ col, row });
  };

  const columnHeaders = createMemo(() => {
    const cols = columns();
    return Array.from({ length: Math.max(cols.length, MIN_COLUMNS) }, (_, i) => ({
      i,
      col: cols[i] ?? null,
    }));
  });

  const displayColumnHeader = (col: Column): string => col.name;

  return (
    <main class="sheet-app">
      <section class="sheet-main">
        <ProjectTopNav title="Database" />

        <Show when={schemaError()}>
          <div class="alert alert-error m-4">
            <span>{schemaError()}</span>
            <button class="btn btn-sm btn-ghost" onClick={() => { const p = currentProject(); if (p) loadSchemas(p.id); }}>Retry</button>
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

          <For each={rows()}>
            {(_, rowIndex) => {
              const dataCols = columns();
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

          {(() => {
            const dataCols = columns();
            const colCount = Math.max(dataCols.length, MIN_COLUMNS);
            const rowCount = rows().length;
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
        </div>
      </footer>
    </main>
  );
}
