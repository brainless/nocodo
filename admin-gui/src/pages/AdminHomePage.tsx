import { For, Show, createEffect, createSignal } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import { ProjectProvider, useProject } from '../contexts/ProjectContext';
import { PromptBox } from '../components/PromptBox';
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

    const response = await fetch(`${API_BASE_URL}/api/agents/project-manager/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ project_id: project.id, message }),
    });

    if (!response.ok) throw new Error(`Failed to initialize project: ${response.status}`);

    navigate(`/projects/${project.id}/manager`);
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
