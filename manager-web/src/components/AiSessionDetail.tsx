import { Component, createSignal, onMount, onCleanup, Show } from 'solid-js';
import { useParams, A } from '@solidjs/router';
import { AiSession, Project, AiSessionStatus } from '../types';
import { useSessions } from '../stores/sessionsStore';
import { apiClient } from '../api';

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

// Status badge component
const StatusBadge: Component<{ status: AiSessionStatus; size?: 'sm' | 'md' }> = (props) => {
  const getStatusColor = () => {
    switch (props.status) {
      case 'completed': return 'bg-green-100 text-green-800 border-green-200';
      case 'running': return 'bg-blue-100 text-blue-800 border-blue-200';
      case 'failed': return 'bg-red-100 text-red-800 border-red-200';
      case 'cancelled': return 'bg-gray-100 text-gray-800 border-gray-200';
      case 'pending': return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      default: return 'bg-gray-100 text-gray-800 border-gray-200';
    }
  };

  const getStatusIcon = () => {
    switch (props.status) {
      case 'completed': return '✓';
      case 'running': return '⟳';
      case 'failed': return '✗';
      case 'cancelled': return '○';
      case 'pending': return '⏳';
      default: return '?';
    }
  };

  const sizeClasses = props.size === 'md' 
    ? 'px-3 py-2 text-sm' 
    : 'px-2 py-1 text-xs';

  return (
    <span class={`${sizeClasses} font-medium rounded-full border ${getStatusColor()} inline-flex items-center gap-1`}>
      <span class={props.status === 'running' ? 'animate-spin' : ''}>{getStatusIcon()}</span>
      {props.status}
    </span>
  );
};

// Live status indicator for running sessions
const LiveStatusIndicator: Component<{ isConnected: boolean }> = (props) => (
  <div class="flex items-center space-x-2">
    <div class={`w-2 h-2 rounded-full ${props.isConnected ? 'bg-green-400 animate-pulse' : 'bg-gray-400'}`}></div>
    <span class="text-sm text-gray-600">
      {props.isConnected ? 'Live updates' : 'No live connection'}
    </span>
  </div>
);

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
  onMount(async () => {
    // First try to get from store, if not available fetch from API
    let sessionData = session();
    if (!sessionData) {
      sessionData = await actions.fetchById(params.id);
    }

    // Fetch associated project if it exists
    if (sessionData?.project_id) {
      fetchProject(sessionData.project_id);
    }

    // Connect to live updates for running sessions
    if (sessionData?.status === 'running') {
      actions.connect(params.id);
      setIsConnected(true);
    }
  });

  // Cleanup: disconnect from live updates when leaving
  onCleanup(() => {
    actions.disconnect(params.id);
    setIsConnected(false);
  });

  // Handle reconnection when session changes to running state
  const handleStatusChange = (newStatus: string) => {
    if (newStatus === 'running' && !isConnected()) {
      actions.connect(params.id);
      setIsConnected(true);
    } else if (newStatus !== 'running' && isConnected()) {
      actions.disconnect(params.id);
      setIsConnected(false);
    }
  };

  return (
    <div class="space-y-6">
      {/* Navigation */}
      <div class="flex items-center space-x-2 text-sm text-gray-500">
        <A href="/ai/sessions" class="hover:text-gray-700">AI Sessions</A>
        <span>›</span>
        <span class="text-gray-900">Session Details</span>
      </div>

      <Show when={store.loading && !session()}>
        <div class="flex justify-center items-center py-8">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
          <span class="ml-2 text-gray-600">Loading session...</span>
        </div>
      </Show>

      <Show when={store.error}>
        <div class="bg-red-50 border border-red-200 rounded-md p-4">
          <div class="flex">
            <div class="ml-3">
              <h3 class="text-sm font-medium text-red-800">Error</h3>
              <div class="mt-2 text-sm text-red-700">
                {store.error}
              </div>
            </div>
          </div>
        </div>
      </Show>

      <Show when={!store.loading && !session()}>
        <div class="text-center py-8">
          <div class="text-gray-400 text-lg mb-2">Session not found</div>
          <A href="/ai/sessions" class="text-blue-600 hover:text-blue-800">
            ← Back to sessions
          </A>
        </div>
      </Show>

      <Show when={session()}>
        <div class="bg-white shadow overflow-hidden sm:rounded-lg">
          {/* Header */}
          <div class="px-4 py-5 sm:px-6 border-b border-gray-200">
            <div class="flex items-center justify-between">
              <div>
                <h1 class="text-2xl font-bold text-gray-900">Session Details</h1>
                <p class="mt-1 max-w-2xl text-sm text-gray-500">
                  Session ID: {session()!.id}
                </p>
              </div>
              <div class="flex flex-col items-end space-y-2">
                <StatusBadge status={session()!.status as AiSessionStatus} size="md" />
                <Show when={session()!.status === 'running'}>
                  <LiveStatusIndicator isConnected={isConnected()} />
                </Show>
              </div>
            </div>
          </div>

          {/* Session Information */}
          <div class="px-4 py-5 sm:p-6">
            <dl class="grid grid-cols-1 gap-x-4 gap-y-6 sm:grid-cols-2">
              {/* Tool */}
              <div>
                <dt class="text-sm font-medium text-gray-500">Tool</dt>
                <dd class="mt-1 text-sm text-gray-900 font-mono bg-gray-50 px-2 py-1 rounded">
                  {session()!.tool_name}
                </dd>
              </div>

              {/* Project */}
              <div>
                <dt class="text-sm font-medium text-gray-500">Project</dt>
                <dd class="mt-1 text-sm text-gray-900">
                  <Show 
                    when={project()} 
                    fallback={session()!.project_id ? `Project ${session()!.project_id}` : 'No Project'}
                  >
                    <A 
                      href={`/projects/${project()!.id}/files`}
                      class="text-blue-600 hover:text-blue-800 font-medium"
                    >
                      {project()!.name}
                    </A>
                    <div class="text-xs text-gray-500 mt-1">
                      {project()!.path}
                    </div>
                  </Show>
                </dd>
              </div>

              {/* Started At */}
              <div>
                <dt class="text-sm font-medium text-gray-500">Started</dt>
                <dd class="mt-1 text-sm text-gray-900">
                  {formatTimestamp(session()!.started_at)}
                </dd>
              </div>

              {/* Ended At */}
              <Show when={session()!.ended_at}>
                <div>
                  <dt class="text-sm font-medium text-gray-500">Ended</dt>
                  <dd class="mt-1 text-sm text-gray-900">
                    {formatTimestamp(session()!.ended_at!)}
                  </dd>
                </div>
              </Show>

              {/* Duration */}
              <div>
                <dt class="text-sm font-medium text-gray-500">Duration</dt>
                <dd class="mt-1 text-sm text-gray-900">
                  {formatDuration(session()!.started_at, session()!.ended_at)}
                  <Show when={!session()!.ended_at}>
                    <span class="text-gray-500"> (ongoing)</span>
                  </Show>
                </dd>
              </div>

              {/* Status with timestamp */}
              <div>
                <dt class="text-sm font-medium text-gray-500">Current Status</dt>
                <dd class="mt-1 text-sm text-gray-900">
                  <StatusBadge status={session()!.status as AiSessionStatus} />
                  <Show when={session()!.status === 'running'}>
                    <div class="mt-1 text-xs text-gray-500">
                      Last updated: {new Date().toLocaleTimeString()}
                    </div>
                  </Show>
                </dd>
              </div>
            </dl>
          </div>

          {/* Prompt Section */}
          <Show when={session()!.prompt}>
            <div class="border-t border-gray-200 px-4 py-5 sm:p-6">
              <dt class="text-sm font-medium text-gray-500 mb-2">Prompt</dt>
              <dd class="text-sm text-gray-900 bg-gray-50 p-4 rounded-md font-mono whitespace-pre-wrap">
                {session()!.prompt}
              </dd>
            </div>
          </Show>

          {/* Project Context */}
          <Show when={session()!.project_context}>
            <div class="border-t border-gray-200 px-4 py-5 sm:p-6">
              <dt class="text-sm font-medium text-gray-500 mb-2">Project Context</dt>
              <dd class="text-sm text-gray-900 bg-gray-50 p-4 rounded-md font-mono whitespace-pre-wrap">
                {session()!.project_context}
              </dd>
            </div>
          </Show>
        </div>
      </Show>

      {/* Back to Sessions */}
      <div class="flex justify-start">
        <A 
          href="/ai/sessions"
          class="inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50"
        >
          ← Back to Sessions
        </A>
      </div>
    </div>
  );
};

export default AiSessionDetail;
