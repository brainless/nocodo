import { For, Show, createSignal } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import { ProjectProvider, useProject } from '../contexts/ProjectContext';

const API_BASE_URL = '';

function formatProjectTimestamp(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
}

const EXAMPLE_PROMPTS = [
  'CRM with leads, companies, contacts, and deal stages',
  'Project tracker with tasks, sprints, and team members',
  'Inventory system with products, suppliers, and stock levels',
  'Support desk with tickets, priorities, and SLA tracking',
];

function HomeContent() {
  const navigate = useNavigate();
  const { createProject } = useProject();
  const [prompt, setPrompt] = createSignal('');
  const [isStarting, setIsStarting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const startWithPrompt = async () => {
    const message = prompt().trim();
    if (!message) return;

    setIsStarting(true);
    setError(null);

    try {
      const project = await createProject(`Project ${formatProjectTimestamp()}`);
      if (!project) throw new Error('Failed to create project');

      const response = await fetch(`${API_BASE_URL}/api/agents/schema-designer/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ project_id: project.id, message }),
      });

      if (!response.ok) throw new Error(`Failed to start schema designer: ${response.status}`);

      navigate(`/projects/${project.id}/db-developer`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start project');
      setIsStarting(false);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      startWithPrompt();
    }
  };

  return (
    <main class="home-page">
      {/* Ambient background blobs */}
      <div class="home-blob home-blob-1" />
      <div class="home-blob home-blob-2" />

      <div class="home-inner">

        {/* Hero */}
        <section class="home-hero">
          <div class="home-badge">
            <span class="home-badge-dot" />
            Nocodo
          </div>
          <h1 class="home-heading">
            Your workflow,<br />built for you in seconds
          </h1>
          <p class="home-subheading">
            Describe your process in plain language. Nocodo designs a spreadsheet workspace — tables, columns, relationships — and opens it ready to use.
          </p>
        </section>

        {/* Prompt box */}
        <section class="home-prompt-section">
          <div class="home-prompt-box">
            <textarea
              class="home-prompt-textarea"
              placeholder="What do you want to build? e.g. A CRM with leads, companies, contacts, tasks, and deal stages."
              rows={4}
              value={prompt()}
              onInput={(e) => setPrompt(e.currentTarget.value)}
              onKeyDown={handleKeyDown}
              disabled={isStarting()}
            />
            <div class="home-prompt-footer">
              <span class="home-prompt-hint">
                <kbd class="kbd kbd-sm">⌘</kbd>
                <kbd class="kbd kbd-sm">↵</kbd>
                to send
              </span>
              <button
                class="home-prompt-btn"
                classList={{ 'home-prompt-btn-loading': isStarting() }}
                onClick={startWithPrompt}
                disabled={isStarting() || !prompt().trim()}
              >
                <Show when={isStarting()}>
                  <span class="loading loading-spinner loading-xs" />
                </Show>
                <Show when={!isStarting()}>
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg>
                </Show>
                Build it
              </button>
            </div>
          </div>

          <Show when={error()}>
            <div class="alert alert-error mt-3 max-w-2xl text-sm">
              <span>{error()}</span>
            </div>
          </Show>

          {/* Example chips */}
          <div class="home-chips">
            <For each={EXAMPLE_PROMPTS}>
              {(ex) => (
                <button
                  class="home-chip"
                  onClick={() => setPrompt(ex)}
                  disabled={isStarting()}
                >
                  {ex}
                </button>
              )}
            </For>
          </div>
        </section>

        {/* Import options */}
        <section class="home-import-section">
          <div class="home-import-header">
            <span class="home-import-divider" />
            <span class="home-import-label">or start from existing data</span>
            <span class="home-import-divider" />
          </div>

          <div class="home-import-grid">
            <div class="home-import-card">
              <div class="home-import-icon home-import-icon-csv">
                <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="8" y1="17" x2="16" y2="17"/><line x1="8" y1="9" x2="10" y2="9"/></svg>
              </div>
              <div class="home-import-text">
                <h3>Upload CSV</h3>
                <p>Import a CSV file and Nocodo will infer your schema from the headers and rows.</p>
              </div>
              <div class="home-import-soon">Soon</div>
            </div>

            <div class="home-import-card">
              <div class="home-import-icon home-import-icon-excel">
                <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="9" y1="15" x2="15" y2="9"/><line x1="15" y1="15" x2="9" y2="9"/></svg>
              </div>
              <div class="home-import-text">
                <h3>Upload Excel</h3>
                <p>Bring in <code>.xlsx</code> workbooks — sheets become tables, columns stay intact.</p>
              </div>
              <div class="home-import-soon">Soon</div>
            </div>

            <div class="home-import-card">
              <div class="home-import-icon home-import-icon-sheets">
                <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="3" y1="15" x2="21" y2="15"/><line x1="9" y1="3" x2="9" y2="21"/><line x1="15" y1="3" x2="15" y2="21"/></svg>
              </div>
              <div class="home-import-text">
                <h3>Connect Google Sheets</h3>
                <p>Link a Google Sheet directly — live sync with your existing collaborative data.</p>
              </div>
              <div class="home-import-soon">Soon</div>
            </div>
          </div>
        </section>

      </div>
    </main>
  );
}

export default function AdminHomePage() {
  return (
    <ProjectProvider>
      <HomeContent />
    </ProjectProvider>
  );
}
