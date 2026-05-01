import { For, createSignal } from 'solid-js';
import { PromptBox } from '../components/PromptBox';
import { ImportCard } from '../components/ImportCard';
import { ContentCard } from '../components/ContentCard';

const DB_EXAMPLES = [
  'CRM with leads, companies, contacts, and deal stages',
  'Project tracker with tasks, sprints, and team members',
  'Inventory system with products, suppliers, and stock levels',
];

const SEARCH_EXAMPLES = [
  'Find all invoices from last month',
  'Show overdue tasks assigned to me',
];

const IconCsv = () => (
  <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
    <polyline points="14 2 14 8 20 8"/>
    <line x1="8" y1="13" x2="16" y2="13"/>
    <line x1="8" y1="17" x2="16" y2="17"/>
    <line x1="8" y1="9" x2="10" y2="9"/>
  </svg>
);

const IconExcel = () => (
  <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
    <polyline points="14 2 14 8 20 8"/>
    <line x1="9" y1="15" x2="15" y2="9"/>
    <line x1="15" y1="15" x2="9" y2="9"/>
  </svg>
);

const IconSheets = () => (
  <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
    <rect x="3" y="3" width="18" height="18" rx="2"/>
    <line x1="3" y1="9" x2="21" y2="9"/>
    <line x1="3" y1="15" x2="21" y2="15"/>
    <line x1="9" y1="3" x2="9" y2="21"/>
    <line x1="15" y1="3" x2="15" y2="21"/>
  </svg>
);

export default function ComponentsGalleryPage() {
  const [lastSubmit, setLastSubmit] = createSignal<string | null>(null);

  return (
    <div class="gallery-page">
      <header class="gallery-header">
        <h1 class="gallery-title">Components Gallery</h1>
        <p class="gallery-subtitle">Visual language reference for Nocodo admin UI</p>
      </header>

      <div class="gallery-body">

        {/* ── PromptBox ── */}
        <section class="gallery-section">
          <div class="gallery-section-label">PromptBox</div>
          <p class="gallery-section-desc">
            Main entry point for natural-language input. Accepts a submit callback, placeholder, example chips, and a CTA label.
          </p>

          <div class="gallery-row">
            <div class="gallery-example">
              <div class="gallery-example-title">Default — with example chips</div>
              <PromptBox
                placeholder="What do you want to build? e.g. A CRM with leads, companies, contacts, tasks, and deal stages."
                examples={DB_EXAMPLES}
                submitLabel="Build it"
                onSubmit={async (v) => {
                  await new Promise((r) => setTimeout(r, 800));
                  setLastSubmit(v);
                }}
              />
              {lastSubmit() && (
                <p class="gallery-output">
                  Submitted: <code>{lastSubmit()}</code>
                </p>
              )}
            </div>
          </div>

          <div class="gallery-row gallery-row--2col">
            <div class="gallery-example">
              <div class="gallery-example-title">Custom placeholder + label, no chips</div>
              <PromptBox
                placeholder="Describe what you want to search for…"
                submitLabel="Search"
                onSubmit={async () => { await new Promise((r) => setTimeout(r, 600)); }}
              />
            </div>

            <div class="gallery-example">
              <div class="gallery-example-title">Short chips variant</div>
              <PromptBox
                placeholder="Ask anything…"
                examples={SEARCH_EXAMPLES}
                submitLabel="Ask"
                onSubmit={async () => { await new Promise((r) => setTimeout(r, 600)); }}
              />
            </div>
          </div>
        </section>

        {/* ── ContentCard ── */}
        <section class="gallery-section">
          <div class="gallery-section-label">ContentCard</div>
          <p class="gallery-section-desc">
            Selectable list item for large content objects. Supports a title, body (2-line clamp), right-aligned meta, and an optional leading slot for avatars or icons. Use inside any vertical list.
          </p>

          {/* Single-card states */}
          <div class="gallery-row gallery-row--2col">
            <div class="gallery-example">
              <div class="gallery-example-title">Title only</div>
              <ContentCard title="CRM with leads and deal stages" onClick={() => {}} />
            </div>
            <div class="gallery-example">
              <div class="gallery-example-title">With body + meta</div>
              <ContentCard
                title="Support desk"
                body="Build a helpdesk with tickets, priorities, SLA tracking, and customer contact history."
                meta="3d ago"
                onClick={() => {}}
              />
            </div>
          </div>

          <div class="gallery-row gallery-row--2col">
            <div class="gallery-example">
              <div class="gallery-example-title">With leading avatar</div>
              <ContentCard
                title="Project 20240501-143022"
                body="Inventory system with products, suppliers, and stock levels"
                meta="just now"
                leading={<div class="project-avatar">I</div>}
                onClick={() => {}}
              />
            </div>
            <div class="gallery-example">
              <div class="gallery-example-title">Selected state</div>
              <ContentCard
                title="CRM with leads and deal stages"
                body="Leads, companies, contacts, tasks, and deal stages across the full pipeline."
                meta="2h ago"
                leading={<div class="project-avatar">C</div>}
                selected
                onClick={() => {}}
              />
            </div>
          </div>

          {/* Selectable list demo */}
          <div class="gallery-row">
            <div class="gallery-example">
              <div class="gallery-example-title">Selectable list — click to select</div>
              {(() => {
                const ITEMS = [
                  { id: 1, title: 'CRM with leads and deal stages', body: 'Leads, companies, contacts, tasks, and deal pipeline stages.', meta: '2h ago', initial: 'C' },
                  { id: 2, title: 'Project tracker', body: 'Tasks, sprints, team members, and burndown across multiple projects.', meta: '1d ago', initial: 'P' },
                  { id: 3, title: 'Inventory system', body: 'Products, suppliers, purchase orders, and real-time stock levels.', meta: '3d ago', initial: 'I' },
                  { id: 4, title: 'Support desk', body: 'Tickets, priorities, SLA tracking, and customer contact history.', meta: '5d ago', initial: 'S' },
                ];
                const [selected, setSelected] = createSignal<number | null>(1);
                return (
                  <div style="display: flex; flex-direction: column; gap: 0.5rem;">
                    <For each={ITEMS}>
                      {(item) => (
                        <ContentCard
                          title={item.title}
                          body={item.body}
                          meta={item.meta}
                          leading={<div class="project-avatar">{item.initial}</div>}
                          selected={selected() === item.id}
                          onClick={() => setSelected(item.id)}
                        />
                      )}
                    </For>
                  </div>
                );
              })()}
            </div>
          </div>
        </section>

        {/* ── ImportCard ── */}
        <section class="gallery-section">
          <div class="gallery-section-label">ImportCard</div>
          <p class="gallery-section-desc">
            Icon-above, text-below card used for data source entry points. Accepts a theme color, icon, title, description, and an optional badge.
          </p>

          <div class="gallery-row gallery-row--3col">
            <div class="gallery-example">
              <div class="gallery-example-title">Blue theme — with badge</div>
              <ImportCard
                theme="blue"
                icon={<IconCsv />}
                title="Upload CSV"
                description="Import a CSV file and Nocodo will infer your schema from the headers and rows."
                badge="Soon"
              />
            </div>

            <div class="gallery-example">
              <div class="gallery-example-title">Green theme — with badge</div>
              <ImportCard
                theme="green"
                icon={<IconExcel />}
                title="Upload Excel"
                description={<>.xlsx workbooks — sheets become tables, columns stay intact.</>}
                badge="Soon"
              />
            </div>

            <div class="gallery-example">
              <div class="gallery-example-title">Orange theme — clickable, no badge</div>
              <ImportCard
                theme="orange"
                icon={<IconSheets />}
                title="Connect Google Sheets"
                description="Link a Google Sheet — live sync with your existing data."
                onClick={() => alert('clicked')}
              />
            </div>
          </div>
        </section>

      </div>
    </div>
  );
}
