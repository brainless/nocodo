import { Component, For, Show, createSignal, onCleanup, onMount } from 'solid-js';
import { A, useNavigate } from '@solidjs/router';
import { Project } from '../types';
import { apiClient } from '../api';
import { useSessions } from '../stores/sessionsStore';

// Utility function to format timestamps
const formatTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp * 1000);
  return date.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
};

// Status badge component for sessions
const SessionStatusBadge: Component<{ status: string }> = props => {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'completed':
        return 'bg-green-100 text-green-800';
      case 'running':
        return 'bg-blue-100 text-blue-800';
      case 'failed':
        return 'bg-red-100 text-red-800';
      case 'cancelled':
        return 'bg-gray-100 text-gray-800';
      default:
        return 'bg-yellow-100 text-yellow-800';
    }
  };

  return (
    <span class={`px-2 py-1 rounded-full text-xs font-medium ${getStatusColor(props.status)}`}>
      {props.status}
    </span>
  );
};

// Projects card component
const ProjectsCard: Component = () => {
  const [projects, setProjects] = createSignal<Project[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const loadProjects = async () => {
    try {
      setLoading(true);
      setError(null);
      const projectList = await apiClient.fetchProjects();
      setProjects(Array.isArray(projectList) ? projectList : []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load projects');
      setProjects([]);
    } finally {
      setLoading(false);
    }
  };

  onMount(loadProjects);

  const recentProjects = () => projects().slice(0, 5);

  return (
    <div class='bg-white rounded-lg shadow-sm border border-gray-200 p-6'>
      <div class='flex items-center justify-between mb-4'>
        <h3 class='text-lg font-semibold text-gray-900'>Recent Projects</h3>
        <A href='/projects' class='text-sm text-blue-600 hover:text-blue-800 font-medium'>
          View all →
        </A>
      </div>

      {loading() ? (
        <div class='flex items-center justify-center py-8'>
          <div class='animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500'></div>
          <span class='ml-2 text-gray-500'>Loading...</span>
        </div>
      ) : error() ? (
        <div class='text-center py-8'>
          <p class='text-red-500 text-sm'>{error()}</p>
        </div>
      ) : projects().length === 0 ? (
        <div class='text-center py-8'>
          <p class='text-gray-500 text-sm mb-3'>No projects yet</p>
          <A
            href='/projects/create'
            class='inline-flex items-center px-3 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700'
          >
            Create your first project
          </A>
        </div>
      ) : (
        <div class='space-y-3'>
          <For each={recentProjects()}>
            {project => (
              <div class='flex items-center justify-between p-3 bg-gray-50 rounded-lg'>
                <div class='flex-1 min-w-0'>
                  <p class='font-medium text-gray-900 truncate'>{project.name}</p>
                  <p class='text-sm text-gray-500 truncate'>{project.path}</p>
                  <div class='flex items-center space-x-2 mt-1'>
                    {project.language && (
                      <span class='text-xs bg-blue-100 text-blue-800 px-2 py-0.5 rounded'>
                        {project.language}
                      </span>
                    )}
                    <span
                      class={`text-xs px-2 py-0.5 rounded ${
                        project.status === 'active'
                          ? 'bg-green-100 text-green-800'
                          : 'bg-gray-100 text-gray-800'
                      }`}
                    >
                      {project.status}
                    </span>
                  </div>
                </div>
                <A
                  href={`/projects/${project.id}/files`}
                  class='ml-3 text-sm text-blue-600 hover:text-blue-800 font-medium'
                >
                  Open
                </A>
              </div>
            )}
          </For>
          {projects().length > 5 && (
            <div class='text-center pt-2'>
              <A href='/projects' class='text-sm text-blue-600 hover:text-blue-800 font-medium'>
                +{projects().length - 5} more projects
              </A>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

// Start AI Session form component (Issue #59)
const StartAiSessionForm: Component = () => {
  const navigate = useNavigate();
  // Known tools
  const knownTools = ['claude', 'gemini', 'openai', 'qwen'];

  const [projects, setProjects] = createSignal<Project[]>([]);
  const [selectedProjectId, setSelectedProjectId] = createSignal<string>('');
  const [toolName, setToolName] = createSignal<string>(knownTools[0]);
  const [customTool, setCustomTool] = createSignal<string>('');
  const [prompt, setPrompt] = createSignal<string>('');
  const [submitting, setSubmitting] = createSignal<boolean>(false);
  const [error, setError] = createSignal<string | null>(null);

  // Dropdown states and refs for project and tool, with click-outside handling
  const [isProjectOpen, setProjectOpen] = createSignal(false);
  const [isToolOpen, setToolOpen] = createSignal(false);
  let projectDdRef: HTMLDivElement | undefined;
  let toolDdRef: HTMLDivElement | undefined;

  const onDocMouseDown = (e: MouseEvent) => {
    const target = e.target as Node;
    if (projectDdRef && !projectDdRef.contains(target)) setProjectOpen(false);
    if (toolDdRef && !toolDdRef.contains(target)) setToolOpen(false);
  };

  onMount(async () => {
    try {
      const list = await apiClient.fetchProjects();
      setProjects(list);
    } catch (e) {
      console.error('Failed to load projects for session form', e);
    }

    document.addEventListener('mousedown', onDocMouseDown);
  });

  onCleanup(() => {
    document.removeEventListener('mousedown', onDocMouseDown);
  });

  const effectiveTool = () => (customTool().trim() ? customTool().trim() : toolName());

  const isValid = () => prompt().trim().length > 0 && effectiveTool().trim().length > 0;

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    if (!isValid()) {
      setError('Please provide a prompt and tool name');
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      const payload: any = {
        tool_name: effectiveTool(),
        prompt: prompt().trim(),
      };
      const pid = selectedProjectId().trim();
      if (pid) payload.project_id = pid;
      const resp = await apiClient.createAiSession(payload);
      const id = resp.session.id;
      // Navigate to detail page
      navigate(`/ai/sessions/${id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start AI session');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div class='bg-white rounded-lg shadow-sm border border-gray-200 p-6'>
      <h3 class='text-lg font-semibold text-gray-900 mb-4'>Start AI Session</h3>
      <form onSubmit={handleSubmit} class='space-y-4'>
        <div>
          <label for='project' class='block text-sm font-medium text-gray-700'>
            Project (optional)
          </label>
          <div class='mt-1 relative' ref={(el: HTMLDivElement) => (projectDdRef = el)}>
            <button
              type='button'
              class='flex items-center justify-between w-full px-3 py-2 text-sm font-medium text-gray-700 hover:text-gray-900 hover:bg-gray-50 rounded-md border border-gray-300'
              onClick={() => setProjectOpen(!isProjectOpen())}
              aria-haspopup='listbox'
              aria-expanded={isProjectOpen()}
            >
              <span class='truncate'>
                {selectedProjectId()
                  ? projects().find(p => p.id === selectedProjectId())?.name ||
                    `Project ${selectedProjectId()}`
                  : 'No Project'}
              </span>
              <svg
                class='w-4 h-4 ml-2 text-gray-500'
                fill='none'
                stroke='currentColor'
                viewBox='0 0 24 24'
              >
                <path
                  stroke-linecap='round'
                  stroke-linejoin='round'
                  stroke-width={2}
                  d='M19 9l-7 7-7-7'
                />
              </svg>
            </button>
            {isProjectOpen() && (
              <div class='absolute left-0 mt-2 w-full bg-white rounded-md shadow-lg border border-gray-200 z-10'>
                <div class='py-1 max-h-60 overflow-auto' role='listbox'>
                  <div
                    role='option'
                    class='block px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 cursor-pointer'
                    onClick={() => {
                      setSelectedProjectId('');
                      setProjectOpen(false);
                    }}
                  >
                    No Project
                  </div>
                  <For each={projects()}>
                    {p => (
                      <div
                        role='option'
                        class='block px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 cursor-pointer'
                        onClick={() => {
                          setSelectedProjectId(p.id);
                          setProjectOpen(false);
                        }}
                      >
                        <div class='font-medium'>{p.name}</div>
                        <div class='text-xs text-gray-500 truncate'>{p.language || ''}</div>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            )}
          </div>
        </div>

        <div class='grid grid-cols-1 md:grid-cols-2 gap-4'>
          <div>
            <label for='tool' class='block text-sm font-medium text-gray-700'>
              Tool
            </label>
            <div class='mt-1 relative' ref={(el: HTMLDivElement) => (toolDdRef = el)}>
              <button
                type='button'
                class='flex items-center justify-between w-full px-3 py-2 text-sm font-medium text-gray-700 hover:text-gray-900 hover:bg-gray-50 rounded-md border border-gray-300'
                onClick={() => setToolOpen(!isToolOpen())}
                aria-haspopup='listbox'
                aria-expanded={isToolOpen()}
              >
                <span class='truncate'>{toolName()}</span>
                <svg
                  class='w-4 h-4 ml-2 text-gray-500'
                  fill='none'
                  stroke='currentColor'
                  viewBox='0 0 24 24'
                >
                  <path
                    stroke-linecap='round'
                    stroke-linejoin='round'
                    stroke-width={2}
                    d='M19 9l-7 7-7-7'
                  />
                </svg>
              </button>
              {isToolOpen() && (
                <div class='absolute left-0 mt-2 w-full bg-white rounded-md shadow-lg border border-gray-200 z-10'>
                  <div class='py-1' role='listbox'>
                    <For each={knownTools}>
                      {t => (
                        <div
                          role='option'
                          class='block px-4 py-2 text-sm text-gray-700 hover:bg-gray-50 cursor-pointer'
                          onClick={() => {
                            setToolName(t);
                            setToolOpen(false);
                          }}
                        >
                          {t}
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              )}
            </div>
            <p class='mt-1 text-xs text-gray-500'>Select a tool or enter a custom one below</p>
          </div>
          <div>
            <label for='customTool' class='block text-sm font-medium text-gray-700'>
              Custom Tool (optional)
            </label>
            <input
              id='customTool'
              type='text'
              placeholder='e.g., my-tool'
              class='mt-1 block w-full px-3 py-2 text-sm text-gray-700 placeholder-gray-400 bg-white border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500'
              value={customTool()}
              onInput={e => setCustomTool(e.currentTarget.value)}
            />
            <p class='mt-1 text-xs text-gray-500'>
              If provided, this will override the selected tool
            </p>
          </div>
        </div>

        <div>
          <label for='prompt' class='block text-sm font-medium text-gray-700'>
            Prompt
          </label>
          <textarea
            id='prompt'
            required
            rows={3}
            class='mt-1 block w-full px-3 py-2 text-sm text-gray-700 placeholder-gray-400 bg-white border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500'
            placeholder='Describe what you want the AI tool to do...'
            value={prompt()}
            onInput={e => setPrompt(e.currentTarget.value)}
          />
        </div>

        <Show when={error()}>
          <div class='text-sm text-red-600'>{error()}</div>
        </Show>

        <div class='flex justify-end'>
          <button
            type='submit'
            disabled={submitting() || !isValid()}
            class={`inline-flex items-center px-4 py-2 text-sm font-medium rounded-md text-white ${
              submitting() || !isValid() ? 'bg-blue-300' : 'bg-blue-600 hover:bg-blue-700'
            }`}
          >
            {submitting() ? 'Starting...' : 'Start Session'}
          </button>
        </div>
      </form>
    </div>
  );
};

// AI Sessions card component
const SessionsCard: Component = () => {
  const { store, actions } = useSessions();
  const [projects, setProjects] = createSignal<Project[]>([]);

  const loadProjects = async () => {
    try {
      const projectList = await apiClient.fetchProjects();
      setProjects(Array.isArray(projectList) ? projectList : []);
    } catch (err) {
      console.error('Failed to load projects:', err);
    }
  };

  const getProjectName = (projectId?: string) => {
    if (!projectId) return 'No Project';
    const project = projects().find(p => p.id === projectId);
    return project?.name || `Project ${projectId}`;
  };

  const recentSessions = () => {
    return [...store.list].sort((a, b) => b.started_at - a.started_at).slice(0, 5);
  };

  onMount(() => {
    actions.fetchList();
    loadProjects();
  });

  return (
    <div class='bg-white rounded-lg shadow-sm border border-gray-200 p-6'>
      <div class='flex items-center justify-between mb-4'>
        <h3 class='text-lg font-semibold text-gray-900'>Recent AI Sessions</h3>
        <A href='/ai/sessions' class='text-sm text-blue-600 hover:text-blue-800 font-medium'>
          View all →
        </A>
      </div>

      {store.loading ? (
        <div class='flex items-center justify-center py-8'>
          <div class='animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500'></div>
          <span class='ml-2 text-gray-500'>Loading...</span>
        </div>
      ) : store.error ? (
        <div class='text-center py-8'>
          <p class='text-red-500 text-sm'>{store.error}</p>
        </div>
      ) : store.list.length === 0 ? (
        <div class='text-center py-8'>
          <p class='text-gray-500 text-sm mb-3'>No AI sessions yet</p>
          <p class='text-xs text-gray-400'>Start your first session using the nocodo CLI</p>
        </div>
      ) : (
        <div class='space-y-3'>
          <For each={recentSessions()}>
            {session => (
              <A
                href={`/ai/sessions/${session.id}`}
                class='block p-3 bg-gray-50 rounded-lg hover:bg-gray-100 transition-colors'
              >
                <div class='flex items-start justify-between'>
                  <div class='flex-1 min-w-0'>
                    <div class='flex items-center space-x-2 mb-1'>
                      <span class='text-sm font-medium text-gray-900'>{session.tool_name}</span>
                      <SessionStatusBadge status={session.status} />
                    </div>
                    <p class='text-sm text-gray-600 truncate mb-1'>{session.prompt}</p>
                    <div class='flex items-center space-x-3 text-xs text-gray-500'>
                      <span>{getProjectName(session.project_id ?? undefined)}</span>
                      <span>•</span>
                      <span>{formatTimestamp(session.started_at)}</span>
                    </div>
                  </div>
                </div>
              </A>
            )}
          </For>
          {store.list.length > 5 && (
            <div class='text-center pt-2'>
              <A href='/ai/sessions' class='text-sm text-blue-600 hover:text-blue-800 font-medium'>
                +{store.list.length - 5} more sessions
              </A>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

// Main Dashboard component
const Dashboard: Component = () => {
  return (
    <div class='space-y-6'>
      {/* Welcome header */}
      <div class='bg-white border-b border-gray-200 -mx-6 -mt-6 px-6 pt-6 pb-4'>
        <div class='flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4'>
          <div>
            <h1 class='text-3xl font-bold text-gray-900'>Dashboard</h1>
            <p class='mt-1 text-sm text-gray-600'>Overview of your projects and AI sessions</p>
          </div>
        </div>
      </div>

      {/* Dashboard cards */}
      <div class='grid grid-cols-1 lg:grid-cols-2 gap-6'>
        <ProjectsCard />
        <SessionsCard />
      </div>

      {/* Start AI Session */}
      <StartAiSessionForm />

    </div>
  );
};

export default Dashboard;
