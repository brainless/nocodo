import { Component, For, Show, createSignal, onCleanup, onMount } from 'solid-js';
import { A, useNavigate } from '@solidjs/router';
import { Project } from '../types';
import type { MessageAuthorType, MessageContentType } from '../types';
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
  // Available tools - LLM agent and external tools
  const knownTools = ['llm-agent', 'claude', 'gemini', 'openai', 'qwen'];

  const [projects, setProjects] = createSignal<Project[]>([]);
  const [selectedProjectId, setSelectedProjectId] = createSignal<string>('');
  const [toolName, setToolName] = createSignal<string>(knownTools[0]);
  const [prompt, setPrompt] = createSignal<string>('');
  const [submitting, setSubmitting] = createSignal<boolean>(false);
  const [error, setError] = createSignal<string | null>(null);

  // PTY options (Issue #58)
  const [usePty, setUsePty] = createSignal<boolean>(false);
  const [terminalCols, setTerminalCols] = createSignal<number>(80);
  const [terminalRows, setTerminalRows] = createSignal<number>(24);

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
      if (usePty()) {
        // Create PTY terminal session (Issue #58)
        const terminalSessionResp = await apiClient.createTerminalSession({
          project_id: selectedProjectId().trim() || undefined,
          tool_name: toolName(),
          prompt: prompt().trim(),
          interactive: true,
          requires_pty: true,
          cols: terminalCols(),
          rows: terminalRows(),
        });

        // Navigate to the terminal session detail page
        // For now, we'll use the same work detail page but it will show terminal UI
        // The response structure is { session: { work_id, ... } }
        const workId = (terminalSessionResp as any)?.session?.work_id;
        if (workId) {
          navigate(`/work/${workId}`);
        } else {
          console.error('No work ID found in terminal session response:', terminalSessionResp);
          setError('Failed to create terminal session: Invalid response format');
        }
      } else {
        // Standard work session workflow
        // 1. Create the work
        const workResp = await apiClient.createWork({
          title: prompt().trim(),
          project_id: selectedProjectId().trim() || null,
        });
        const workId = workResp.work.id;

        // 2. Add the initial message
        const messageResp = await apiClient.addMessageToWork(workId, {
          content: prompt().trim(),
          content_type: 'text' as MessageContentType,
          author_type: 'user' as MessageAuthorType,
          author_id: null, // Assuming user is not logged in
        });
        const messageId = messageResp.message.id;

        // 3. Create the AI session
        await apiClient.createAiSession(workId, {
          message_id: messageId,
          tool_name: toolName(),
        });

        // Navigate to the new work's detail page
        navigate(`/work/${workId}`);
      }
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

        {/* PTY Options (Issue #58) */}
        <div class='border-t border-gray-200 pt-4'>
          <div class='flex items-center space-x-3 mb-4'>
            <input
              id='use-pty'
              type='checkbox'
              checked={usePty()}
              onInput={e => setUsePty(e.currentTarget.checked)}
              class='h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded'
            />
            <label for='use-pty' class='text-sm font-medium text-gray-700'>
              Use interactive terminal (PTY mode)
            </label>
          </div>

          <Show when={usePty()}>
            <div class='bg-blue-50 border border-blue-200 rounded-md p-4 mb-4'>
              <div class='flex items-start'>
                <svg
                  class='w-5 h-5 text-blue-400 mt-0.5 mr-3'
                  fill='currentColor'
                  viewBox='0 0 20 20'
                >
                  <path
                    fill-rule='evenodd'
                    d='M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z'
                    clip-rule='evenodd'
                  ></path>
                </svg>
                <div>
                  <h4 class='text-sm font-medium text-blue-800'>Interactive Terminal Mode</h4>
                  <p class='text-sm text-blue-700 mt-1'>
                    This will launch the AI tool in a full interactive terminal with support for
                    ANSI colors, cursor positioning, and real-time input/output. Perfect for tools
                    that provide rich terminal interfaces.
                  </p>
                </div>
              </div>
            </div>

            <div class='grid grid-cols-2 gap-4'>
              <div>
                <label for='terminal-cols' class='block text-sm font-medium text-gray-700 mb-1'>
                  Terminal Width (columns)
                </label>
                <input
                  id='terminal-cols'
                  type='number'
                  min='20'
                  max='200'
                  value={terminalCols()}
                  onInput={e => setTerminalCols(parseInt(e.currentTarget.value) || 80)}
                  class='w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500'
                />
              </div>
              <div>
                <label for='terminal-rows' class='block text-sm font-medium text-gray-700 mb-1'>
                  Terminal Height (rows)
                </label>
                <input
                  id='terminal-rows'
                  type='number'
                  min='10'
                  max='100'
                  value={terminalRows()}
                  onInput={e => setTerminalRows(parseInt(e.currentTarget.value) || 24)}
                  class='w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500'
                />
              </div>
            </div>
          </Show>
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
