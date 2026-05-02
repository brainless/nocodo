import { For, type JSX } from 'solid-js';

export interface SheetColumn<T> {
  key: string;
  header: string;
  width?: string;
  render: (row: T) => JSX.Element | string;
}

interface SheetProps<T> {
  columns: SheetColumn<T>[];
  rows: T[];
  rowKey: (row: T) => string | number;
  onRowClick?: (row: T) => void;
  emptyRows?: number;
}

export function Sheet<T>(props: SheetProps<T>) {
  const emptyRows = () => Math.max(0, (props.emptyRows ?? 50) - props.rows.length);

  const gridTemplateColumns = () => {
    const rowHeaderWidth = '56px';
    const colWidths = props.columns.map(c => c.width ?? '1fr').join(' ');
    return `${rowHeaderWidth} ${colWidths}`;
  };

  return (
    <div
      class="grid-wrap"
      style={{ 'grid-template-columns': gridTemplateColumns() }}
    >
      {/* Corner */}
      <div class="grid-corner" />

      {/* Column headers */}
      <For each={props.columns}>
        {(col) => (
          <div class="column-header column-header-field">
            <span class="truncate">{col.header}</span>
          </div>
        )}
      </For>

      {/* Data rows */}
      <For each={props.rows}>
        {(row, rowIndex) => (
          <>
            <div class="row-header">{rowIndex() + 1}</div>
            <For each={props.columns}>
              {(col) => (
                <div
                  class={`cell${props.onRowClick ? ' cursor-pointer hover:bg-base-200' : ''}`}
                  onClick={() => props.onRowClick?.(row)}
                >
                  {col.render(row)}
                </div>
              )}
            </For>
          </>
        )}
      </For>

      {/* Empty padding rows */}
      <For each={Array.from({ length: emptyRows() }, (_, i) => i)}>
        {(offset) => {
          const rowNum = props.rows.length + offset + 1;
          return (
            <>
              <div class="row-header">{rowNum}</div>
              <For each={props.columns}>
                {() => <div class="cell" />}
              </For>
            </>
          );
        }}
      </For>
    </div>
  );
}
