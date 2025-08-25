import { A, useNavigate, useParams } from '@solidjs/router';
import { Component, For, Show, createSignal, onMount } from 'solid-js';
import { AiSession, Project } from '../types';
import { apiClient } from '../api';
import FileBrowser from './FileBrowser';

interface ProjectComponentInfo {
  id: string;
  project_id: string;
  name: string;
  path: string; // relative
  language: string;
  framework?: string | null;
  created_at: number;
}

const Tabs = ['work', 'code', 'project', 'automation', 'about'] as const;

type TabKey = typeof Tabs[number];

const TabLabel: Record<TabKey, string> = {
  work: 'Work',
  code: 'Code',
  project: 'Project Management',
  automation: 'Automation',
  about: 'About',
};

const ProjectDetails: Component = () => {
  const params = useParams();
  const navigate = useNavigate();

  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  // current tab from URL
  const currentTab = () => {
    const raw = (params as unknown as { tab?: string }).tab || 'work';
    return (['work', 'code', 'project', 'automation', 'about'] as const).includes(raw as any)
      ? (raw as TabKey)
      : 'work';
  };

  const [project, setProject] = createSignal<Project | null>(null);
  const [components, setComponents] = createSignal<ProjectComponentInfo[]>([]);
  const [sessions, setSessions] = createSignal<AiSession[]>([]);

  const projectId = () => params.id;

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);

      const pid = projectId();
      if (!pid) {
        navigate('/projects');
        return;
      }

      // Fetch project details + components
      const details = await apiClient.fetchProjectDetails(pid);
      setProject(details.project);
      setComponents(details.components as ProjectComponentInfo[]);

      // Fetch sessions and filter by project
      const all = await apiClient.listSessions();
      setSessions(all.filter(s => s.project_id === pid));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load project details');
    } finally {
      setLoading(false);
    }
  };

  onMount(loadData);

  const isActive = (k: TabKey) => (currentTab() === k ? 'border-blue-600 text-blue-600' : 'border-transparent text-gray-600 hover:text-gray-800');

  return (
    <div class="space-y-6">
      {/* Header */}
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-bold text-gray-900">{project()?.name ?? 'Project'}</h1>
          <p class="text-sm text-gray-600 mt-1">Project Dashboard</p>
        </div>
        <div class="flex items-center gap-2">
          <A href="/projects" class="px-3 py-2 text-sm bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition-colors">
            ← Back to Projects
          </A>
        </div>
      </div>

      {/* Tabs */}
      <div class="border-b border-gray-200">
        <nav class="-mb-px flex space-x-6" aria-label="Tabs">
          <For each={Tabs as unknown as TabKey[]}>
            {key => (
              <A
                href={`/projects/${projectId()}/${key}`}
                class={`whitespace-nowrap border-b-2 px-3 py-2 text-sm font-medium ${isActive(key)}`}
              >
                {TabLabel[key]}
              </A>
            )}
          </For>
        </nav>
      </div>

      {/* Content */}
      <Show when={!loading()} fallback={<div class="text-gray-500">Loading...</div>}>
        <Show when={!error()} fallback={<div class="p-3 bg-red-50 border border-red-200 text-red-700 text-sm rounded">{error()}</div>}>
          {/* Work Tab */}
          <Show when={currentTab() === 'work'}>
            <div class="space-y-3">
              <Show when={sessions().length > 0} fallback={<div class="text-gray-500">No sessions found for this project.</div>}>
                <div class="space-y-2">
                  <For each={sessions()}>
                    {s => (
                      <A href={`/ai/sessions/${s.id}`} class="block p-3 bg-white border border-gray-200 rounded hover:bg-gray-50">
                        <div class="flex items-center justify-between">
                          <div>
                            <div class="text-sm font-medium text-gray-900">{s.tool_name}</div>
                            <div class="text-xs text-gray-500 truncate max-w-xl">{s.prompt}</div>
                          </div>
                          <div class="text-xs text-gray-500">{new Date(s.started_at * 1000).toLocaleString()}</div>
                        </div>
                      </A>
                    )}
                  </For>
                </div>
              </Show>
            </div>
          </Show>

          {/* Code Tab */}
          <Show when={currentTab() === 'code'}>
            <div class="border border-gray-200 rounded-lg p-2">
              {/* Use existing FileBrowser; compactness handled by container padding */}
              <FileBrowser projectId={projectId()} projectName={project()?.name} />
            </div>
          </Show>

          {/* Project Management Tab */}
          <Show when={currentTab() === 'project'}>
            <div class="text-gray-600 text-sm">Coming soon</div>
          </Show>

          {/* Automation Tab */}
          <Show when={currentTab() === 'automation'}>
            <div class="text-gray-600 text-sm">Coming soon</div>
          </Show>

          {/* About Tab */}
          <Show when={currentTab() === 'about'}>
            <div class="space-y-4">
              <div class="bg-white border border-gray-200 rounded p-4">
                <h3 class="text-lg font-semibold mb-2">Project</h3>
                <div class="text-sm text-gray-700">
                  <div><span class="font-medium">Name:</span> {project()?.name}</div>
                  <div><span class="font-medium">Path:</span> {project()?.path}</div>
                  <div><span class="font-medium">Language:</span> {project()?.language ?? 'Unknown'}</div>
                  <div><span class="font-medium">Framework:</span> {project()?.framework ?? 'N/A'}</div>
                  <div><span class="font-medium">Status:</span> {project()?.status}</div>
                </div>
              </div>

              <div class="bg-white border border-gray-200 rounded p-4">
                <h3 class="text-lg font-semibold mb-3">Components</h3>
                <Show when={components().length > 0} fallback={<div class="text-sm text-gray-600">No components detected</div>}>
                  <div class="space-y-2">
                    <For each={components()}>
                      {c => (
                        <div class="p-3 border border-gray-200 rounded">
                          <div class="flex items-center justify-between">
                            <div class="font-medium text-gray-900">{c.name}</div>
                            <div class="text-xs text-gray-500">{c.language}{c.framework ? ` • ${c.framework}` : ''}</div>
                          </div>
                          <div class="text-xs text-gray-500 mt-1">/{c.path}</div>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>
              </div>
            </div>
          </Show>
        </Show>
      </Show>
    </div>
  );
};

export default ProjectDetails;
