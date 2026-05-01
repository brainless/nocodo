import { Show, createSignal } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import { ProjectProvider, useProject } from '../contexts/ProjectContext';

const API_BASE_URL = '';

function formatProjectTimestamp(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
}

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

      navigate(`/projects/${project.id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start project');
      setIsStarting(false);
    }
  };

  return (
    <main class="min-h-screen bg-gradient-to-b from-base-200 to-base-100 p-6 md:p-10">
      <div class="mx-auto flex w-full max-w-6xl flex-col gap-8">
        <section class="hero rounded-box border border-base-300 bg-base-100 shadow-xl">
          <div class="hero-content w-full flex-col gap-8 p-8 lg:flex-row lg:items-start lg:justify-between">
            <div class="max-w-2xl">
              <div class="badge badge-success badge-outline mb-4">Nocodo</div>
              <h1 class="text-4xl font-bold leading-tight md:text-5xl">
                Start a project from your workflow
              </h1>
              <p class="mt-4 text-base-content/80">
                Describe your process in plain language. Nocodo turns that prompt into a spreadsheet-style software workspace using the schema designer agent.
              </p>
            </div>

            <div class="card w-full max-w-xl border border-base-300 bg-base-100">
              <div class="card-body gap-4">
                <h2 class="card-title">Prompt-based setup</h2>
                <p class="text-sm text-base-content/70">Tell us what you want to build and we will create a new project and open it in Sheets.</p>
                <textarea
                  class="textarea textarea-bordered h-32"
                  placeholder="Example: I need a CRM with leads, companies, contacts, tasks, and deal stages."
                  value={prompt()}
                  onInput={(e) => setPrompt(e.currentTarget.value)}
                  disabled={isStarting()}
                />
                <Show when={error()}>
                  <div class="alert alert-error text-sm"><span>{error()}</span></div>
                </Show>
                <button class="btn btn-success" onClick={startWithPrompt} disabled={isStarting() || !prompt().trim()}>
                  <Show when={isStarting()}>
                    <span class="loading loading-spinner loading-xs mr-2"></span>
                  </Show>
                  Start with Prompt
                </button>
              </div>
            </div>
          </div>
        </section>

        <section>
          <h2 class="mb-4 text-xl font-semibold">Other ways to start (coming soon)</h2>
          <div class="grid gap-4 md:grid-cols-3">
            <div class="card border border-base-300 bg-base-100">
              <div class="card-body">
                <h3 class="card-title text-lg">Upload CSV</h3>
                <p class="text-sm text-base-content/70">Bring your workflow from a CSV file.</p>
                <button class="btn btn-outline btn-disabled">Upload CSV</button>
              </div>
            </div>
            <div class="card border border-base-300 bg-base-100">
              <div class="card-body">
                <h3 class="card-title text-lg">Upload Excel</h3>
                <p class="text-sm text-base-content/70">Import from `.xlsx` spreadsheets.</p>
                <button class="btn btn-outline btn-disabled">Upload Excel</button>
              </div>
            </div>
            <div class="card border border-base-300 bg-base-100">
              <div class="card-body">
                <h3 class="card-title text-lg">Connect Google Sheet</h3>
                <p class="text-sm text-base-content/70">Link a Google Sheet as your starting point.</p>
                <button class="btn btn-outline btn-disabled">Connect Google Sheet</button>
              </div>
            </div>
          </div>
          <p class="mt-3 text-sm text-base-content/60">CSV, Excel, and Google Sheets import UI is placeholder-only in this version.</p>
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
