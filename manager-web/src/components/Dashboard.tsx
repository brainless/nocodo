import { Component, For, Show, createSignal, onCleanup, onMount } from 'solid-js';
import { A, useNavigate } from '@solidjs/router';
import { Project } from '../types';
import type { MessageAuthorType, MessageContentType } from '../types';
import type { SupportedModel } from '../types/generated';
import { apiClient } from '../api';
import { useSessions } from '../stores/sessionsStore';

import AiSessionCard from './AiSessionCard';
import { AddExistingProjectForm } from './CreateProjectForm';

// Start AI Session form component (Issue #59)
const StartAiSessionForm: Component = () => {
  const navigate = useNavigate();
  // Only use LLM agent as per issue #110 - no tool selection needed
  const toolName = 'llm-agent';

  const [projects, setProjects] = createSignal<Project[]>([]);
  const [selectedProjectId, setSelectedProjectId] = createSignal<number | null>(null);
  const [models, setModels] = createSignal<SupportedModel[]>([]);
  const [selectedModel, setSelectedModel] = createSignal<string>('');
  const [prompt, setPrompt] = createSignal<string>('');
  const [submitting, setSubmitting] = createSignal<boolean>(false);
  const [error, setError] = createSignal<string | null>(null);

  // Dropdown states and refs for project, with click-outside handling
  const [isProjectOpen, setProjectOpen] = createSignal(false);
  let projectDdRef: HTMLDivElement | undefined;

  const onDocMouseDown = (e: MouseEvent) => {
    const target = e.target as Node;
    if (projectDdRef && !projectDdRef.contains(target)) setProjectOpen(false);
  };

  onMount(async () => {
    try {
      const list = await apiClient.fetchProjects();
      setProjects(list);
    } catch (e) {
      console.error('Failed to load projects for session form', e);
    }

    try {
      const modelsList = await apiClient.fetchSupportedModels();
      setModels(modelsList.models);
      // Set default model if available
      if (modelsList.models.length > 0) {
        setSelectedModel(modelsList.models[0].model_id);
      }
    } catch (e) {
      console.error('Failed to load supported models', e);
    }

    document.addEventListener('mousedown', onDocMouseDown);
  });

  onCleanup(() => {
    document.removeEventListener('mousedown', onDocMouseDown);
  });

  const isValid = () => prompt().trim().length > 0;

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    if (!isValid()) {
      setError('Please provide a prompt');
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      // Standard work session workflow with LLM agent only
      // 1. Create the work
      const workResp = await apiClient.createWork({
        title: prompt().trim(),
        project_id: selectedProjectId() ?? undefined,
        model: selectedModel().trim() || null,
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

      // 3. Create the AI session with LLM agent
      await apiClient.createAiSession(workId, {
        message_id: messageId.toString(),
        tool_name: toolName,
      });

      // Navigate to the new work's detail page
      navigate(`/work/${workId}`);
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
                  ? projects().find(p => p.id === Number(selectedProjectId()))?.name ||
                    `Project ${selectedProjectId()!}`
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
                      setSelectedProjectId(null);
                      setProjectOpen(false);
                    }}
                  >
                    No Project
                  </div>
                  <For each={projects()}>
                    {(p: Project) => (
                      <div
                        role='option'
                        class='block px-4 py-2 text-sm text-gray-700 hover:bg-muted cursor-pointer'
                        onClick={() => {
                          setSelectedProjectId(Number(p.id));
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
          <label for='model' class='block text-sm font-medium text-gray-700'>
            Model (optional)
          </label>
          <div class='mt-1'>
            <select
              id='model'
              class='block w-full px-3 py-2 text-sm text-gray-700 bg-white border border-border rounded-md hover:bg-muted focus:outline-none focus:ring-2 focus:ring-ring focus:border-ring'
              value={selectedModel()}
              onInput={e => setSelectedModel(e.currentTarget.value)}
            >
              <option value=''>Default Model</option>
              <For each={models()}>
                {model => (
                  <option value={model.model_id}>
                    {model.name} ({model.provider})
                  </option>
                )}
              </For>
            </select>
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
        <div class='grid grid-cols-1 gap-6'>
          <For each={recentSessions()}>
            {session => {
              const project = session.project_id
                ? projects().find(p => p.id === Number(session.project_id))
                : undefined;
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

// Project CTA component - only shown when no projects exist
const ProjectCTA: Component = () => {
  const [showAddExistingModal, setShowAddExistingModal] = createSignal(false);
  // Check if projects exist after adding one
  const handleProjectAdded = () => {
    setShowAddExistingModal(false);
  };

  return (
    <>
      <div class='bg-gradient-to-r from-blue-50 to-indigo-50 rounded-lg border border-blue-200 p-8'>
        <div class='text-center'>
          <div class='text-blue-600 text-6xl mb-4'>📁</div>
          <h2 class='text-2xl font-bold text-gray-900 mb-2'>Create or add a Project</h2>
          <p class='text-gray-600 mb-6 max-w-md mx-auto'>
            Get started by creating your first project or adding an existing one to organize your
            work.
          </p>
          <div class='flex flex-col sm:flex-row gap-3 justify-center'>
            <A
              href='/projects/create'
              class='inline-flex items-center px-6 py-3 text-base font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 transition-colors'
            >
              Create New Project
            </A>
            <button
              onClick={() => setShowAddExistingModal(true)}
              class='inline-flex items-center px-6 py-3 text-base font-medium text-blue-600 bg-white border border-blue-300 rounded-lg hover:bg-blue-50 transition-colors'
            >
              Add Existing Project
            </button>
          </div>
        </div>
      </div>

      {/* Modal for Add Existing Project Form */}
      <Show when={showAddExistingModal()}>
        <div
          class='fixed top-0 left-0 right-0 bottom-0 bg-black bg-opacity-50 flex items-center justify-center'
          style='z-index: 1000;'
        >
          <div class='bg-white rounded-lg shadow-xl p-6 max-w-md w-full mx-4 max-h-[90vh] overflow-y-auto relative'>
            <div class='flex justify-between items-center mb-4'>
              <h3 class='text-lg font-semibold'>Add Existing Project</h3>
              <button
                onClick={() => setShowAddExistingModal(false)}
                class='text-gray-400 hover:text-gray-600 p-1 rounded-full hover:bg-gray-100 transition-colors'
              >
                <svg class='w-5 h-5' fill='none' stroke='currentColor' viewBox='0 0 24 24'>
                  <path
                    stroke-linecap='round'
                    stroke-linejoin='round'
                    stroke-width={2}
                    d='M6 18L18 6M6 6l12 12'
                  />
                </svg>
              </button>
            </div>
            <AddExistingProjectForm onProjectAdded={handleProjectAdded} />
          </div>
        </div>
      </Show>
    </>
  );
};

// Main Dashboard component
const Dashboard: Component = () => {
  const [hasProjects, setHasProjects] = createSignal<boolean | null>(null);

  // Check if projects exist
  const checkProjects = async () => {
    try {
      const projectList = await apiClient.fetchProjects();
      setHasProjects(Array.isArray(projectList) && projectList.length > 0);
    } catch (err) {
      console.error('Failed to check projects:', err);
      setHasProjects(false); // Default to showing CTA on error
    }
  };

  onMount(checkProjects);

  return (
    <div class='space-y-8 mt-8'>
      {/* Start Work Session - first section */}
      <StartAiSessionForm />

      {/* Project CTA - only if no projects exist */}
      <Show when={hasProjects() === false}>
        <ProjectCTA />
      </Show>

      {/* Recent Work - second section */}
      <SessionsCard />
    </div>
  );
};

export default Dashboard;
