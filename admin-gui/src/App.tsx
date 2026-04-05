import { For } from 'solid-js';

const menuItems = ['File', 'Edit', 'View', 'Insert', 'Format', 'Data', 'Tools', 'Help'];
const columns = Array.from({ length: 12 }, (_, index) => String.fromCharCode(65 + index));
const rows = Array.from({ length: 28 }, (_, index) => index + 1);
const sheets = ['Leads', 'Pipeline', 'Forecast', 'Invoices', 'Archive'];

export default function App() {
  return (
    <main class="sheet-app">
      <header class="menu-strip">
        <div class="menu-title">Nocodo Sheets</div>
        <nav class="menu-items">
          <For each={menuItems}>{(item) => <button class="menu-item">{item}</button>}</For>
        </nav>
        <div class="menu-status">Synced</div>
      </header>

      <section class="sheet-main">
        <div class="formula-strip">
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
    </main>
  );
}
