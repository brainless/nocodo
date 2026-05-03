import { For, Show, createEffect, createSignal } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import { ProjectProvider, useProject } from '../contexts/ProjectContext';
import { PromptBox } from '../components/PromptBox';
import { ImportCard } from '../components/ImportCard';
import { ContentCard } from '../components/ContentCard';
import type { Project, ListTasksResponse } from '../types/api';

const API_BASE_URL = '';

function formatProjectTimestamp(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
}

function timeAgo(unixSeconds: number): string {
  const diff = Math.floor(Date.now() / 1000) - unixSeconds;
  if (diff < 60) return 'just now';
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  if (diff < 7 * 86400) return `${Math.floor(diff / 86400)}d ago`;
  return new Date(unixSeconds * 1000).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function projectInitial(name: string): string {
  return name.replace(/^Project\s+/i, '').charAt(0).toUpperCase() || 'P';
}

const EXAMPLE_PROMPTS = [
  'CRM with leads, companies, contacts, and deal stages',
  'Project tracker with tasks, sprints, and team members',
  'Inventory system with products, suppliers, and stock levels',
  'Support desk with tickets, priorities, and SLA tracking',
];

type ProjectWithContext = { project: Project; firstPrompt: string | null };

function RecentProjects() {
  const navigate = useNavigate();
  const { projects, isLoading: projectsLoading } = useProject();
  const [items, setItems] = createSignal<ProjectWithContext[]>([]);
  const [loading, setLoading] = createSignal(true);

  createEffect(() => {
    const ps = projects();
    if (projectsLoading()) return;

    setLoading(true);
    void (async () => {
      try {
        const taskData = await Promise.all(
          ps.map(p =>
            fetch(`${API_BASE_URL}/api/agents/tasks?project_id=${p.id}`)
              .then(r => r.json() as Promise<ListTasksResponse>)
              .catch(() => ({ tasks: [] } as ListTasksResponse))
          )
        );

        const withContext = ps.map((project, i) => {
          const tasks = taskData[i].tasks ?? [];
          const latest = tasks
            .filter(t => t.assigned_to_agent === 'schema_designer')
            .sort((a, b) => b.created_at - a.created_at)[0];
          return { project, firstPrompt: latest?.source_prompt ?? null };
        });

        setItems(withContext);
      } finally {
        setLoading(false);
      }
    })();
  });

  return (
    <Show when={!projectsLoading() && (loading() || items().length > 0)}>
      <section class="home-recent-section">
        <div class="home-recent-header">
          <span class="home-recent-title">Recent projects</span>
        </div>
        <div class="home-recent-list">
          <Show when={loading()} fallback={
            <For each={items()}>
              {({ project, firstPrompt }) => (
                <ContentCard
                  title={project.name}
                  body={firstPrompt}
                  meta={timeAgo(project.created_at)}
                  leading={<div class="project-avatar">{projectInitial(project.name)}</div>}
                  onClick={() => navigate(`/projects/${project.id}/db-developer`)}
                />
              )}
            </For>
          }>
            <div class="home-recent-skeleton" />
            <div class="home-recent-skeleton" style="opacity: 0.6" />
            <div class="home-recent-skeleton" style="opacity: 0.35" />
          </Show>
        </div>
      </section>
    </Show>
  );
}

function HomeContent() {
  const navigate = useNavigate();
  const { createProject } = useProject();
  const [error, setError] = createSignal<string | null>(null);

  const startWithPrompt = async (message: string) => {
    setError(null);

    const project = await createProject(`Project ${formatProjectTimestamp()}`);
    if (!project) throw new Error('Failed to create project');

    const response = await fetch(`${API_BASE_URL}/api/agents/pm/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ project_id: project.id, message }),
    });

    if (!response.ok) throw new Error(`Failed to initialize project: ${response.status}`);

    navigate(`/projects/${project.id}/project-manager`);
  };

  const handleSubmit = async (message: string) => {
    try {
      await startWithPrompt(message);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start project');
    }
  };

  return (
    <main class="home-page">
      <div class="home-blob home-blob-1" />
      <div class="home-blob home-blob-2" />

      <div class="home-inner">

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

        <section class="home-prompt-section">
          <PromptBox
            placeholder="What do you want to build? e.g. A CRM with leads, companies, contacts, tasks, and deal stages."
            examples={EXAMPLE_PROMPTS}
            submitLabel="Build it"
            onSubmit={handleSubmit}
          />
          <Show when={error()}>
            <div class="alert alert-error mt-3 max-w-2xl text-sm">
              <span>{error()}</span>
            </div>
          </Show>
        </section>

        <section class="home-import-section">
          <div class="home-import-header">
            <span class="home-import-divider" />
            <span class="home-import-label">or start from existing data</span>
            <span class="home-import-divider" />
          </div>
          <div class="home-import-grid">
            <ImportCard
              theme="blue"
              icon={<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="8" y1="13" x2="16" y2="13"/><line x1="8" y1="17" x2="16" y2="17"/><line x1="8" y1="9" x2="10" y2="9"/></svg>}
              title="Upload CSV"
              description="Import a CSV file and Nocodo will infer your schema from the headers and rows."
              badge="Soon"
            />
            <ImportCard
              theme="green"
              icon={<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="9" y1="15" x2="15" y2="9"/><line x1="15" y1="15" x2="9" y2="9"/></svg>}
              title="Upload Excel"
              description={<>Bring in <code>.xlsx</code> workbooks — sheets become tables, columns stay intact.</>}
              badge="Soon"
            />
            <ImportCard
              theme="orange"
              icon={<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="3" y1="15" x2="21" y2="15"/><line x1="9" y1="3" x2="9" y2="21"/><line x1="15" y1="3" x2="15" y2="21"/></svg>}
              title="Connect Google Sheets"
              description="Link a Google Sheet directly — live sync with your existing collaborative data."
              badge="Soon"
            />
          </div>
        </section>

        <RecentProjects />

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
