import { Component, createSignal, createEffect, onMount, For, Show } from 'solid-js';
import { A } from '@solidjs/router';
import { AiSession, Project, AiSessionStatus } from '../types';
import { useSessions } from '../stores/sessionsStore';
import { apiClient } from '../api';

// Utility function to format timestamps
const formatTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp * 1000); // Convert from Unix timestamp
  return date.toLocaleString();
};

// Utility function to format duration
const formatDuration = (startedAt: number, endedAt?: number): string => {
  const start = new Date(startedAt * 1000);
  const end = endedAt ? new Date(endedAt * 1000) : new Date();
  const durationMs = end.getTime() - start.getTime();
  
  const minutes = Math.floor(durationMs / 60000);
  const seconds = Math.floor((durationMs % 60000) / 1000);
  
  if (minutes > 0) {
    return `${minutes}m ${seconds}s`;
  }
  return `${seconds}s`;
};

// Status badge component
const StatusBadge: Component<{ status: AiSessionStatus }> = (props) => {
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

  return (
    <span class={`px-2 py-1 text-xs font-medium rounded-full border ${getStatusColor()}`}>
      {props.status}
    </span>
  );
};

// Filter component
interface FiltersProps {
  toolFilter: string;
  statusFilter: string;
  onToolChange: (tool: string) => void;
  onStatusChange: (status: string) => void;
  tools: string[];
}

const Filters: Component<FiltersProps> = (props) => {
  const statuses: AiSessionStatus[] = ['pending', 'running', 'completed', 'failed', 'cancelled'];

  return (
    <div class="flex flex-wrap gap-4 mb-6 p-4 bg-gray-50 rounded-lg">
      <div class="flex flex-col">
        <label class="text-sm font-medium text-gray-700 mb-1">Tool</label>
        <select
          value={props.toolFilter}
          onInput={(e) => props.onToolChange(e.currentTarget.value)}
          class="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">All Tools</option>
          <For each={props.tools}>
            {(tool) => <option value={tool}>{tool}</option>}
          </For>
        </select>
      </div>
      
      <div class="flex flex-col">
        <label class="text-sm font-medium text-gray-700 mb-1">Status</label>
        <select
          value={props.statusFilter}
          onInput={(e) => props.onStatusChange(e.currentTarget.value)}
          class="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
        >
          <option value="">All Statuses</option>
          <For each={statuses}>
            {(status) => <option value={status}>{status}</option>}
          </For>
        </select>
      </div>
    </div>
  );
};

const AiSessionsList: Component = () => {
  const { store, actions } = useSessions();
  const [projects, setProjects] = createSignal<Project[]>([]);
  const [toolFilter, setToolFilter] = createSignal('');
  const [statusFilter, setStatusFilter] = createSignal('');

  // Fetch projects for displaying project names
  const fetchProjects = async () => {
    try {
      const projectsList = await apiClient.fetchProjects();
      setProjects(projectsList);
    } catch (err) {
      console.error('Failed to fetch projects:', err);
    }
  };

  // Get project name by ID
  const getProjectName = (projectId?: string) => {
    if (!projectId) return 'No Project';
    const project = projects().find(p => p.id === projectId);
    return project?.name || `Project ${projectId}`;
  };

  // Filter sessions based on current filters
  const filteredSessions = () => {
    let sessions = store.list;
    
    if (toolFilter()) {
      sessions = sessions.filter(session => session.tool_name === toolFilter());
    }
    
    if (statusFilter()) {
      sessions = sessions.filter(session => session.status === statusFilter());
    }
    
    return sessions.sort((a, b) => b.started_at - a.started_at); // Sort by newest first
  };

  // Get unique tools for filter dropdown
  const uniqueTools = () => {
    const tools = store.list.map(session => session.tool_name);
    return [...new Set(tools)];
  };

  onMount(() => {
    actions.fetchList();
    fetchProjects();
  });

  return (
    <div class="space-y-6">
      <div class="flex justify-between items-center">
        <h1 class="text-2xl font-bold text-gray-900">AI Sessions</h1>
        <span class="text-sm text-gray-500">
          {filteredSessions().length} session{filteredSessions().length !== 1 ? 's' : ''}
        </span>
      </div>

      <Filters 
        toolFilter={toolFilter()}
        statusFilter={statusFilter()}
        onToolChange={setToolFilter}
        onStatusChange={setStatusFilter}
        tools={uniqueTools()}
      />

      <Show when={store.loading}>
        <div class="flex justify-center items-center py-8">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
          <span class="ml-2 text-gray-600">Loading sessions...</span>
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

      <Show when={!store.loading && filteredSessions().length === 0}>
        <div class="text-center py-8">
          <div class="text-gray-400 text-lg mb-2">No AI sessions found</div>
          <Show when={toolFilter() || statusFilter()}>
            <button
              onClick={() => { setToolFilter(''); setStatusFilter(''); }}
              class="text-blue-600 hover:text-blue-800 text-sm"
            >
              Clear filters
            </button>
          </Show>
        </div>
      </Show>

      <Show when={!store.loading && filteredSessions().length > 0}>
        <div class="bg-white shadow overflow-hidden sm:rounded-md">
          <ul class="divide-y divide-gray-200">
            <For each={filteredSessions()}>
              {(session) => (
                <li>
                  <A 
                    href={`/ai/sessions/${session.id}`}
                    class="block hover:bg-gray-50 px-4 py-4 sm:px-6"
                  >
                    <div class="flex items-center justify-between">
                      <div class="flex items-center space-x-3">
                        <div class="flex-shrink-0">
                          <StatusBadge status={session.status as AiSessionStatus} />
                        </div>
                        <div class="min-w-0 flex-1">
                          <div class="flex items-center space-x-3">
                            <p class="text-sm font-medium text-gray-900 truncate">
                              {session.tool_name}
                            </p>
                            <p class="text-sm text-gray-500">
                              {getProjectName(session.project_id)}
                            </p>
                          </div>
                          <p class="mt-1 text-sm text-gray-600 line-clamp-2">
                            {session.prompt || 'No prompt'}
                          </p>
                        </div>
                      </div>
                      <div class="flex-shrink-0 text-right">
                        <p class="text-sm text-gray-500">
                          {formatTimestamp(session.started_at)}
                        </p>
                        <p class="text-xs text-gray-400">
                          Duration: {formatDuration(session.started_at, session.ended_at)}
                        </p>
                      </div>
                    </div>
                  </A>
                </li>
              )}
            </For>
          </ul>
        </div>
      </Show>
    </div>
  );
};

export default AiSessionsList;
