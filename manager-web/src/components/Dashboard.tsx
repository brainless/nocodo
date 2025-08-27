import { Component, For, Show, createSignal, onCleanup, onMount } from 'solid-js';
import { A, useNavigate } from '@solidjs/router';
import { Project } from '../types';
import { apiClient } from '../api';
import { useSessions } from '../stores/sessionsStore';
import ProjectCard from './ProjectCard';
import AiSessionCard from './AiSessionCard';

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
    <div>
      {/* Section header outside the white box */}
      <div class='flex items-center justify-between mb-6'>
        <h2 class='text-xl font-semibold text-gray-900'>Recent Projects</h2>
        <A href='/projects' class='text-sm text-blue-600 hover:text-blue-800 font-medium'>
          View all →
        </A>
      </div>

      {/* Project cards grid - no outer container */}
      {loading() ? (
        <div class='grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6'>
          {[1, 2, 3].map(() => (
            <div class='animate-pulse'>
              <div class='bg-gray-200 rounded-xl h-48'></div>
            </div>
          ))}
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
        <div class='grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6'>
          <For each={recentProjects()}>{project => <ProjectCard project={project} />}</For>

          {/* Show more projects if there are more than 5 */}
          {projects().length > 5 && (
            <div class='flex items-center justify-center'>
              <A
                href='/projects'
                class='p-6 border-2 border-dashed border-gray-300 rounded-lg hover:border-gray-400 transition-colors text-center'
              >
                <div class='text-gray-500'>
                  <span class='text-sm font-medium'>+{projects().length - 5} more projects</span>
                  <p class='text-xs mt-1'>View all projects</p>
                </div>
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


  const isValid = () => prompt().trim().length > 0 && toolName().trim().length > 0;

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
        tool_name: toolName(),
        prompt: prompt().trim(),
      };
      const pid = selectedProjectId().trim();
      if (pid) payload.project_id = pid;
      const resp = await apiClient.createAiSession(payload);
      const id = resp.session.id;
      // Navigate to detail page
      navigate(`/work/${id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start AI session');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div class='bg-white rounded-lg shadow-sm border border-gray-200 p-6'>
      <h3 class='text-lg font-semibold text-gray-900 mb-4'>What would you like to Work on?</h3>
      <form onSubmit={handleSubmit} class='space-y-4'>
        <div>
          <label for='prompt' class='block text-sm font-medium text-gray-700'>
            Prompt
          </label>
          <textarea
            id='prompt'
            required
            rows={3}
            class='mt-1 block w-full px-3 py-2 text-sm text-gray-700 placeholder-gray-400 bg-white border border-border rounded-md hover:bg-muted focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring'
            placeholder='Describe what you want to work on...'
            value={prompt()}
            onInput={e => setPrompt(e.currentTarget.value)}
          />
        </div>

        <div class='grid grid-cols-1 md:grid-cols-2 gap-4'>
          <div>
            <label for='project' class='block text-sm font-medium text-gray-700'>
              Project (optional)
            </label>
            <div class='mt-1 relative' ref={(el: HTMLDivElement) => (projectDdRef = el)}>
              <button
                type='button'
                class='flex items-center justify-between w-full px-3 py-2 text-sm font-medium text-gray-700 hover:text-gray-900 hover:bg-muted rounded-md border border-border'
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
                      class='block px-4 py-2 text-sm text-gray-700 hover:bg-muted cursor-pointer'
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
                          class='block px-4 py-2 text-sm text-gray-700 hover:bg-muted cursor-pointer'
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
          <div>
            <label for='tool' class='block text-sm font-medium text-gray-700'>
              Tool
            </label>
            <div class='mt-1 relative' ref={(el: HTMLDivElement) => (toolDdRef = el)}>
              <button
                type='button'
                class='flex items-center justify-between w-full px-3 py-2 text-sm font-medium text-gray-700 hover:text-gray-900 hover:bg-muted rounded-md border border-border'
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
                          class='block px-4 py-2 text-sm text-gray-700 hover:bg-muted cursor-pointer'
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
          </div>
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
            {submitting() ? 'Starting...' : 'Start Work Session'}
          </button>
        </div>
      </form>
    </div>
  );
};

// Work card component
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

  const recentSessions = () => {
    return [...store.list].sort((a, b) => b.started_at - a.started_at).slice(0, 5);
  };

  onMount(() => {
    actions.fetchList();
    loadProjects();
  });

  return (
    <div>
      {/* Section header outside the white box */}
      <div class='flex items-center justify-between mb-6'>
        <h2 class='text-xl font-semibold text-gray-900'>Recent Work</h2>
        <A href='/work' class='text-sm text-blue-600 hover:text-blue-800 font-medium'>
          View all →
        </A>
      </div>

      {/* Work cards grid - no outer container */}
      {store.loading ? (
        <div class='grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6'>
          {[1, 2, 3].map(() => (
            <div class='animate-pulse'>
              <div class='bg-gray-200 rounded-lg h-48'></div>
            </div>
          ))}
        </div>
      ) : store.error ? (
        <div class='text-center py-8'>
          <p class='text-red-500 text-sm'>{store.error}</p>
        </div>
      ) : store.list.length === 0 ? (
        <div class='text-center py-8'>
          <div class='mx-auto max-w-md'>
            <div class='text-gray-400 text-6xl mb-4'>🤖</div>
            <h3 class='text-lg font-medium text-gray-900 mb-2'>No work yet</h3>
            <p class='text-gray-500 mb-4'>Start your first work session using the nocodo CLI</p>
          </div>
        </div>
      ) : (
        <div class='grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6'>
          <For each={recentSessions()}>
            {session => {
              const project = projects().find(p => p.id === session.project_id);
              return <AiSessionCard session={session} project={project} showPrompt={true} />;
            }}
          </For>

          {/* Show more sessions if there are more than 5 */}
          {store.list.length > 5 && (
            <div class='flex items-center justify-center'>
              <A
                href='/work'
                class='p-6 border-2 border-dashed border-gray-300 rounded-lg hover:border-gray-400 transition-colors text-center'
              >
                <div class='text-gray-500'>
                  <span class='text-sm font-medium'>+{store.list.length - 5} more work items</span>
                  <p class='text-xs mt-1'>View all work</p>
                </div>
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
    <div class='space-y-8'>
      {/* Recent Projects - stacked vertically */}
      <ProjectsCard />

      {/* Recent Work - stacked vertically */}
      <SessionsCard />

      {/* Start Work Session */}
      <StartAiSessionForm />
    </div>
  );
};

export default Dashboard;
