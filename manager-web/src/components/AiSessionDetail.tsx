import { Component, Show, createSignal, onCleanup, onMount } from 'solid-js';
import { A, useParams } from '@solidjs/router';
import { AiSession, AiSessionStatus, Project } from '../types';
import { useSessions } from '../stores/sessionsStore';
import { apiClient } from '../api';
import { ProjectBadge, StatusBadge, ToolIcon } from './SessionRow';
import SessionTimeline from './SessionTimeline';

// Utility function to format timestamps
const formatTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp * 1000);
  return date.toLocaleString();
};

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

// Live status indicator for running sessions
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

const AiSessionDetail: Component = () => {
  const params = useParams<{ id: string }>();
  const { store, actions } = useSessions();
  const [project, setProject] = createSignal<Project | null>(null);
  const [isConnected, setIsConnected] = createSignal(false);

  const session = () => store.byId[params.id];

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
          sessionData = await actions.fetchById(params.id);
        }

        // Only proceed if component is still mounted
        if (!isMounted || !sessionData) return;

        // Fetch associated project if it exists
        if (sessionData.project_id) {
          fetchProject(sessionData.project_id).catch(console.error);
        }

        // Connect to live updates for running sessions
        if (sessionData.status === 'running') {
          actions.connect(params.id);
          setIsConnected(true);
        }
      } catch (error) {
        console.error('Failed to load session data:', error);
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
              href='/ai/sessions'
              class='text-gray-500 hover:text-gray-700 focus:outline-none focus:underline cursor-pointer'
            >
              AI Sessions
            </A>
          </li>
          <li>
            <span class='text-gray-400' aria-hidden='true'>
              ›
            </span>
          </li>
          <li>
            <span class='text-gray-900 font-medium' aria-current='page'>
              Session Details
            </span>
          </li>
        </ol>
      </nav>

      <Show when={store.loading && !session()}>
        <div class='flex justify-center items-center py-8'>
          <div class='animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500'></div>
          <span class='ml-2 text-gray-600'>Loading session...</span>
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
          <div class='text-gray-400 text-lg mb-2'>Session not found</div>
          <A
            href='/ai/sessions'
            class='text-blue-600 hover:text-blue-800 cursor-pointer focus:outline-none focus:underline'
          >
            ← Back to sessions
          </A>
        </div>
      </Show>

      <Show when={session()}>
        <div class='grid grid-cols-1 lg:grid-cols-3 gap-6'>
          {/* Main content */}
          <div class='lg:col-span-2 space-y-6'>
            {/* Session header card */}
            <div class='bg-white shadow-sm border border-gray-200 rounded-lg overflow-hidden'>
              <div class='px-6 py-4 border-b border-gray-200 bg-gradient-to-r from-blue-50 to-indigo-50'>
                <div class='flex items-start justify-between'>
                  <div class='flex-1'>
                    <h1 class='text-2xl font-bold text-gray-900 mb-2'>Session Details</h1>
                    <div class='flex items-center space-x-4 mb-3'>
                      <StatusBadge
                        status={session()!.status as AiSessionStatus}
                        size='md'
                        showIcon={true}
                      />
                      <ToolIcon toolName={session()!.tool_name} />
                    </div>
                    <p class='text-sm text-gray-600'>
                      Session ID:{' '}
                      <code class='bg-gray-100 px-2 py-1 rounded font-mono text-xs'>
                        {session()!.id}
                      </code>
                    </p>
                  </div>
                  <Show when={session()!.status === 'running'}>
                    <LiveStatusIndicator
                      connectionStatus={actions.getConnectionStatus(params.id)}
                    />
                  </Show>
                </div>
              </div>

              {/* Session Information */}
              <div class='px-6 py-4'>
                <dl class='grid grid-cols-1 gap-x-6 gap-y-4 sm:grid-cols-2'>
                  {/* Project */}
                  <div>
                    <dt class='text-sm font-medium text-gray-500 mb-1'>Project</dt>
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
                        <div class='space-y-2'>
                          <ProjectBadge project={project()} />
                          <A
                            href={`/projects/${project()!.id}/files`}
                            class='text-sm text-blue-600 hover:text-blue-800 font-medium block focus:outline-none focus:underline'
                          >
                            View project files →
                          </A>
                          <div class='text-xs text-gray-500'>{project()!.path}</div>
                        </div>
                      </Show>
                    </dd>
                  </div>

                  {/* Started At */}
                  <div>
                    <dt class='text-sm font-medium text-gray-500 mb-1'>Started</dt>
                    <dd class='text-sm text-gray-900'>
                      <time dateTime={new Date(session()!.started_at * 1000).toISOString()}>
                        {formatTimestamp(session()!.started_at)}
                      </time>
                    </dd>
                  </div>

                  {/* Ended At */}
                  <Show when={session()!.ended_at}>
                    <div>
                      <dt class='text-sm font-medium text-gray-500 mb-1'>Ended</dt>
                      <dd class='text-sm text-gray-900'>
                        <time dateTime={new Date(session()!.ended_at! * 1000).toISOString()}>
                          {formatTimestamp(session()!.ended_at!)}
                        </time>
                      </dd>
                    </div>
                  </Show>

                  {/* Duration */}
                  <div>
                    <dt class='text-sm font-medium text-gray-500 mb-1'>Duration</dt>
                    <dd class='text-sm text-gray-900'>
                      {formatDuration(session()!.started_at, session()!.ended_at ?? undefined)}
                      <Show when={!session()!.ended_at}>
                        <span class='text-blue-600 font-medium'> (ongoing)</span>
                      </Show>
                    </dd>
                  </div>
                </dl>
              </div>
            </div>

            {/* Prompt Section */}
            <Show when={session()!.prompt}>
              <div class='bg-white shadow-sm border border-gray-200 rounded-lg overflow-hidden'>
                <div class='px-6 py-4 border-b border-gray-200 bg-gray-50'>
                  <h3 class='text-lg font-medium text-gray-900'>Session Prompt</h3>
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

            {/* Project Context */}
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
            {/* Timeline */}
            <SessionTimeline session={session()!} />
          </div>
        </div>
      </Show>

      {/* Action buttons */}
      <div class='flex justify-between items-center pt-6 border-t border-gray-200'>
        <A
          href='/ai/sessions'
          class='inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 cursor-pointer'
        >
          <span class='mr-2' aria-hidden='true'>
            ←
          </span>
          Back to Sessions
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

export default AiSessionDetail;
