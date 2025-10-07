import { Component, For, Show, createSignal, onCleanup, onMount } from 'solid-js';
import { A, useParams } from '@solidjs/router';
import { AiSessionStatus, Project } from '../types';
import { apiClient } from '../api';
import { useSessionOutputs, useSessions } from '../stores/sessionsStore';
import { ProjectBadge, ToolIcon, WorkWidget } from './SessionRow';
import SessionTimeline from './SessionTimeline';
import ToolCallProgress from './ToolCallProgress';

// Utility function to format duration
const formatDuration = (startedAt: number, endedAt?: number): string => {
  const start = new Date(startedAt * 1000);
  const end = endedAt ? new Date(endedAt * 1000) : new Date();
  const durationMs = end.getTime() - start.getTime();

  const hours = Math.floor(durationMs / 3600000);
  const minutes = Math.floor((durationMs % 3600000) / 60000);
  const seconds = Math.floor((durationMs % 60000) / 1000);

  if (hours > 0) {
    return `${hours}h ${minutes}m ${seconds}s`;
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds}s`;
  }
  return `${seconds}s`;
};

// Live status indicator for running work
const LiveStatusIndicator: Component<{
  connectionStatus: 'connected' | 'disconnected' | 'error' | 'fallback';
}> = props => {
  const getStatusInfo = () => {
    switch (props.connectionStatus) {
      case 'connected':
        return {
          color: 'bg-green-400',
          animation: 'animate-pulse',
          text: 'Live updates',
          description: 'Real-time WebSocket connection active',
        };
      case 'fallback':
        return {
          color: 'bg-yellow-400',
          animation: 'animate-pulse',
          text: 'Polling updates',
          description: 'Using polling fallback (5s intervals)',
        };
      case 'error':
        return {
          color: 'bg-red-400',
          animation: '',
          text: 'Connection error',
          description: 'Unable to connect to live updates',
        };
      default:
        return {
          color: 'bg-gray-400',
          animation: '',
          text: 'No live connection',
          description: 'Not receiving live updates',
        };
    }
  };

  const statusInfo = getStatusInfo();

  return (
    <div class='flex flex-col items-end space-y-1'>
      <div class='flex items-center space-x-2'>
        <div class={`w-2 h-2 rounded-full ${statusInfo.color} ${statusInfo.animation}`}></div>
        <span class='text-sm text-gray-600'>{statusInfo.text}</span>
      </div>
      <span class='text-xs text-gray-500' title={statusInfo.description}>
        {statusInfo.description}
      </span>
    </div>
  );
};

// Parse output content to handle structured JSON format
const parseOutputContent = (content: string): { text: string; toolCalls?: any[] } => {
  try {
    const parsed = JSON.parse(content);
    if (parsed && typeof parsed === 'object' && 'text' in parsed) {
      return {
        text: parsed.text || '',
        toolCalls: parsed.tool_calls || undefined,
      };
    }
  } catch {
    // Not JSON, return as-is
  }
  return { text: content };
};

// Output panel component - simplified card-based display
const OutputPanel: Component<{ sessionId: string }> = props => {
  const outputs = useSessionOutputs(props.sessionId);

  return (
    <Show
      when={outputs.get().length > 0}
      fallback={
        <div class='text-center py-8 text-gray-500'>
          <div class='text-gray-400 text-lg mb-2'>No output yet</div>
          <p class='text-sm'>Output will appear here as the work progresses</p>
        </div>
      }
    >
      <div class='space-y-3'>
        <For each={outputs.get()}>
          {chunk => {
            const parsed = parseOutputContent(chunk.content);
            return (
              <div
                class={`bg-gray-50 border rounded-lg p-4 ${
                  chunk.stream === 'stderr' ? 'border-red-200 bg-red-50' : 'border-gray-200'
                }`}
              >
                <div class='flex items-start justify-between mb-2'>
                  <span
                    class={`text-xs font-medium px-2 py-1 rounded ${
                      chunk.stream === 'stderr'
                        ? 'bg-red-100 text-red-800'
                        : 'bg-gray-100 text-gray-800'
                    }`}
                  >
                    {chunk.stream === 'stderr' ? 'Error' : 'Output'}
                  </span>
                </div>
                <div class='text-sm text-gray-900 whitespace-pre-wrap font-mono leading-relaxed'>
                  {parsed.text}
                </div>
                <Show when={parsed.toolCalls && parsed.toolCalls.length > 0}>
                  <div class='mt-3 pt-3 border-t border-gray-300 space-y-2'>
                    <For each={parsed.toolCalls}>
                      {toolCall => (
                        <div class='text-xs text-gray-700'>
                          <span class='font-semibold'>{toolCall.function?.name || 'tool'}</span>
                          <span class='text-gray-500'>({toolCall.function?.arguments || ''})</span>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>
              </div>
            );
          }}
        </For>
      </div>
    </Show>
  );
};

const AiSessionDetail: Component = () => {
  const params = useParams<{ id: string }>();
  const { store, actions } = useSessions();
  const [project, setProject] = createSignal<Project | null>(null);
  const [, setIsConnected] = createSignal(false);

  const session = () => store.byId[params.id];
  const toolCalls = () => actions.getToolCalls(params.id);

  // Fetch project details when session is loaded
  const fetchProject = async (projectId: string) => {
    try {
      const projectData = await apiClient.fetchProject(projectId);
      setProject(projectData);
    } catch (err) {
      console.error('Failed to fetch project:', err);
      setProject(null);
    }
  };

  // Load session data and connect to live updates
  onMount(() => {
    // Use a flag to prevent operations after unmount
    let isMounted = true;

    // Load session data safely
    const loadSessionData = async () => {
      try {
        // First try to get from store, if not available fetch from API
        let sessionData = session();
        if (!sessionData && isMounted) {
          const fetchedData = await actions.fetchById(params.id);
          if (!fetchedData) {
            console.error('Work not found');
            return;
          }
          sessionData = fetchedData;
        }

        // Only proceed if component is still mounted
        if (!isMounted || !sessionData) return;

        // Fetch associated project if it exists
        if (sessionData.project_id) {
          fetchProject(sessionData.project_id).catch(console.error);
        }

        // Seed outputs via HTTP, then connect for live chunks
        await actions.fetchOutputs(params.id);

        // Connect to live updates for active sessions
        const st = sessionData.status;
        const isFinished = st === 'completed' || st === 'failed' || st === 'cancelled';
        if (!isFinished) {
          actions.connect(params.id);
          setIsConnected(true);
        }
      } catch (error) {
        console.error('Failed to load work data:', error);
      }
    };

    // Start loading asynchronously
    loadSessionData();

    // Cleanup function
    return () => {
      isMounted = false;
    };
  });

  // Cleanup: disconnect from live updates when leaving
  onCleanup(() => {
    try {
      actions.disconnect(params.id);
      setIsConnected(false);
    } catch (error) {
      // Ignore cleanup errors during unmounting
      console.warn('Error during cleanup:', error);
    }
  });

  return (
    <div class='space-y-6'>
      {/* Breadcrumb navigation */}
      <nav class='flex' aria-label='Breadcrumb'>
        <ol role='list' class='flex items-center space-x-2 text-sm'>
          <li>
            <A
              href='/work'
              class='text-gray-500 hover:text-gray-700 focus:outline-none focus:underline cursor-pointer'
            >
              Work Management
            </A>
          </li>
          <li>
            <span class='text-gray-400' aria-hidden='true'>
              ›
            </span>
          </li>
          <li>
            <span class='text-gray-900 font-medium' aria-current='page'>
              Work Details
            </span>
          </li>
        </ol>
      </nav>

      <Show when={store.loading && !session()}>
        <div class='flex justify-center items-center py-8'>
          <div class='animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500'></div>
          <span class='ml-2 text-gray-600'>Loading work...</span>
        </div>
      </Show>

      <Show when={store.error}>
        <div class='bg-red-50 border border-red-200 rounded-md p-4'>
          <div class='flex'>
            <div class='ml-3'>
              <h3 class='text-sm font-medium text-red-800'>Error</h3>
              <div class='mt-2 text-sm text-red-700'>{store.error}</div>
            </div>
          </div>
        </div>
      </Show>

      <Show when={!store.loading && !session()}>
        <div class='text-center py-8'>
          <div class='text-gray-400 text-lg mb-2'>Work not found</div>
          <A
            href='/work'
            class='text-blue-600 hover:text-blue-800 cursor-pointer focus:outline-none focus:underline'
          >
            ← Back to work
          </A>
        </div>
      </Show>

      <Show when={session()}>
        <div class='grid grid-cols-1 lg:grid-cols-3 gap-6'>
          {/* Main content */}
          <div class='lg:col-span-2 space-y-6'>
            {/* Prompt Section - moved to first */}
            <Show when={session()!.prompt}>
              <div class='bg-white shadow-sm border border-gray-200 rounded-lg overflow-hidden'>
                <div class='px-6 py-4 border-b border-gray-200 bg-gray-50'>
                  <h3 class='text-lg font-medium text-gray-900'>Work Prompt</h3>
                  <p class='text-sm text-gray-600 mt-1'>The original request sent to the AI tool</p>
                </div>
                <div class='px-6 py-4'>
                  <div class='bg-gray-50 border border-gray-200 rounded-lg p-4'>
                    <pre class='text-sm text-gray-900 whitespace-pre-wrap font-sans leading-relaxed'>
                      {session()!.prompt}
                    </pre>
                  </div>
                </div>
              </div>
            </Show>

            {/* Output panel - shows live output from LLM agent */}
            <OutputPanel sessionId={params.id} />

            {/* Project Context - remains last */}
            <Show when={session()!.project_context}>
              <div class='bg-white shadow-sm border border-gray-200 rounded-lg overflow-hidden'>
                <div class='px-6 py-4 border-b border-gray-200 bg-gray-50'>
                  <h3 class='text-lg font-medium text-gray-900'>Project Context</h3>
                  <p class='text-sm text-gray-600 mt-1'>
                    Additional context provided to the AI tool
                  </p>
                </div>
                <div class='px-6 py-4'>
                  <div class='bg-gray-50 border border-gray-200 rounded-lg p-4'>
                    <pre class='text-sm text-gray-900 whitespace-pre-wrap font-mono leading-relaxed'>
                      {session()!.project_context}
                    </pre>
                  </div>
                </div>
              </div>
            </Show>
          </div>

          {/* Sidebar */}
          <div class='lg:col-span-1 space-y-6'>
            {/* Work Summary */}
            <div class='bg-white shadow-sm border border-gray-200 rounded-lg overflow-hidden'>
              <div class='px-6 py-4 border-b border-gray-200 bg-gray-50'>
                <h3 class='text-lg font-medium text-gray-900'>Work Summary</h3>
                <p class='text-sm text-gray-600 mt-1'>Overview of work details</p>
              </div>
              <div class='px-6 py-4 space-y-4'>
                {/* Status and Tool */}
                <div class='flex items-center justify-between'>
                  <div class='flex flex-col space-y-2'>
                    <WorkWidget
                      type='status'
                      value={session()!.status}
                      status={session()!.status as AiSessionStatus}
                    />
                  </div>
                  <ToolIcon toolName={session()!.tool_name} model={session()!.model} />
                </div>

                {/* Work ID */}
                <div>
                  <dt class='text-xs font-medium text-gray-500 mb-1'>Work ID</dt>
                  <dd>
                    <code class='bg-gray-100 px-2 py-1 rounded font-mono text-xs text-gray-900'>
                      {session()!.id}
                    </code>
                  </dd>
                </div>

                {/* Project */}
                <div>
                  <dt class='text-xs font-medium text-gray-500 mb-1'>Project</dt>
                  <dd>
                    <Show
                      when={project()}
                      fallback={
                        <ProjectBadge
                          project={null}
                          projectId={session()!.project_id ?? undefined}
                        />
                      }
                    >
                      <div class='space-y-1'>
                        <ProjectBadge project={project()} />
                        <A
                          href={`/projects/${project()!.id}/files`}
                          class='text-xs text-blue-600 hover:text-blue-800 font-medium block focus:outline-none focus:underline'
                        >
                          View project files →
                        </A>
                        <div class='text-xs text-gray-500 truncate' title={project()!.path}>
                          {project()!.path}
                        </div>
                      </div>
                    </Show>
                  </dd>
                </div>

                {/* Duration */}
                <div>
                  <dt class='text-xs font-medium text-gray-500 mb-1'>Duration</dt>
                  <dd class='text-sm text-gray-900'>
                    {formatDuration(session()!.started_at, session()!.ended_at ?? undefined)}
                    <Show when={!session()!.ended_at}>
                      <span class='text-blue-600 font-medium'> (ongoing)</span>
                    </Show>
                  </dd>
                </div>

                {/* Live Status for running work */}
                <Show
                  when={
                    session()!.status !== 'completed' &&
                    session()!.status !== 'failed' &&
                    session()!.status !== 'cancelled'
                  }
                >
                  <div class='pt-2 border-t border-gray-100'>
                    <LiveStatusIndicator
                      connectionStatus={actions.getConnectionStatus(params.id)}
                    />
                  </div>
                </Show>
              </div>
            </div>

            {/* Tool Calls */}
            <Show when={toolCalls().length > 0}>
              <div class='mt-6'>
                <h3 class='text-lg font-medium text-gray-900 mb-4'>Tool Execution</h3>
                <div class='space-y-2'>
                  <For each={toolCalls()}>
                    {toolCall => <ToolCallProgress toolCall={toolCall} />}
                  </For>
                </div>
              </div>
            </Show>

            {/* Timeline */}
            <SessionTimeline session={session()!} />

            {/* Optional input box to send content to running session */}
            <Show when={session()!.status === 'running'}>
              <SessionInputBox sessionId={params.id} />
            </Show>
          </div>
        </div>
      </Show>

      {/* Action buttons */}
      <div class='flex justify-between items-center pt-6 border-t border-gray-200'>
        <A
          href='/work'
          class='inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 cursor-pointer'
        >
          <span class='mr-2' aria-hidden='true'>
            ←
          </span>
          Back to Work Management
        </A>
        <Show when={session() && session()!.project_id}>
          <A
            href={`/projects/${session()!.project_id}/files`}
            class='inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 cursor-pointer'
          >
            View Project
            <span class='ml-2' aria-hidden='true'>
              →
            </span>
          </A>
        </Show>
      </div>
    </div>
  );
};

// Work input box component (Issue #59)
const SessionInputBox: Component<{ sessionId: string }> = props => {
  const [content, setContent] = createSignal('');
  const [sending, setSending] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const send = async (e: Event) => {
    e.preventDefault();
    if (!content().trim()) return;
    setSending(true);
    setError(null);
    try {
      await apiClient.sendAiInput(props.sessionId, content().trim());
      setContent('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to send input');
    } finally {
      setSending(false);
    }
  };

  return (
    <div class='bg-white rounded-lg border border-gray-200 p-4'>
      <h3 class='text-md font-medium text-gray-900 mb-2'>Send Input</h3>
      <p class='text-sm text-gray-600 mb-3'>
        Send a follow-up message to the running session (tool must read stdin).
      </p>
      <form onSubmit={send} class='space-y-2'>
        <textarea
          rows={3}
          class='w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500'
          placeholder='Type your message...'
          value={content()}
          onInput={e => setContent(e.currentTarget.value)}
        />
        <Show when={error()}>
          <div class='text-sm text-red-600'>{error()}</div>
        </Show>
        <div class='flex justify-end'>
          <button
            type='submit'
            disabled={sending() || !content().trim()}
            class={`inline-flex items-center px-3 py-1.5 text-sm font-medium rounded-md text-white ${
              sending() || !content().trim() ? 'bg-blue-300' : 'bg-blue-600 hover:bg-blue-700'
            }`}
          >
            {sending() ? 'Sending...' : 'Send'}
          </button>
        </div>
      </form>
    </div>
  );
};

export default AiSessionDetail;
