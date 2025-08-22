import { Component, createSignal, onMount, For, Show } from 'solid-js';
import { AiSession, Project, AiSessionStatus } from '../types';
import { useSessions } from '../stores/sessionsStore';
import { apiClient } from '../api';
import SessionRow, { StatusBadge } from './SessionRow';

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


// Filter component with improved accessibility
interface FiltersProps {
  toolFilter: string;
  statusFilter: string;
  onToolChange: (tool: string) => void;
  onStatusChange: (status: string) => void;
  tools: string[];
  totalSessions: number;
}

const Filters: Component<FiltersProps> = (props) => {
  const statuses: AiSessionStatus[] = ['pending', 'running', 'completed', 'failed', 'cancelled'];

  return (
    <div class="bg-white border border-gray-200 rounded-lg p-4 mb-6 shadow-sm">
      <h2 class="text-lg font-medium text-gray-900 mb-4">Filter Sessions</h2>
      <div class="flex flex-wrap gap-6">
        <div class="flex flex-col min-w-0 flex-1">
          <label 
            for="tool-filter"
            class="text-sm font-medium text-gray-700 mb-2"
          >
            Tool
          </label>
          <select
            id="tool-filter"
            value={props.toolFilter}
            onInput={(e) => props.onToolChange(e.currentTarget.value)}
            class="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-white"
            aria-describedby="tool-filter-description"
          >
            <option value="">All Tools ({props.tools.length + 1})</option>
            <For each={props.tools}>
              {(tool) => (
                <option value={tool}>
                  {tool} ({props.totalSessions})
                </option>
              )}
            </For>
          </select>
          <span id="tool-filter-description" class="text-xs text-gray-500 mt-1">
            Filter sessions by AI tool
          </span>
        </div>
        
        <div class="flex flex-col min-w-0 flex-1">
          <label 
            for="status-filter"
            class="text-sm font-medium text-gray-700 mb-2"
          >
            Status
          </label>
          <select
            id="status-filter"
            value={props.statusFilter}
            onInput={(e) => props.onStatusChange(e.currentTarget.value)}
            class="px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 bg-white"
            aria-describedby="status-filter-description"
          >
            <option value="">All Statuses</option>
            <For each={statuses}>
              {(status) => (
                <option value={status} class="capitalize">
                  {status}
                </option>
              )}
            </For>
          </select>
          <span id="status-filter-description" class="text-xs text-gray-500 mt-1">
            Filter sessions by completion status
          </span>
        </div>
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
      {/* Page header with improved styling */}
      <div class="bg-white border-b border-gray-200 -mx-6 -mt-6 px-6 pt-6 pb-4">
        <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <div>
            <h1 class="text-3xl font-bold text-gray-900">AI Sessions</h1>
            <p class="mt-1 text-sm text-gray-600">
              Monitor and manage your AI-assisted development sessions
            </p>
          </div>
          <div class="flex items-center space-x-4">
            <div class="text-sm text-gray-500">
              <span class="font-medium text-gray-900">
                {filteredSessions().length}
              </span>
              {' '}
              session{filteredSessions().length !== 1 ? 's' : ''}
              {(toolFilter() || statusFilter()) && (
                <span class="ml-2">
                  (filtered from {store.list.length})
                </span>
              )}
            </div>
          </div>
        </div>
      </div>

      <Filters 
        toolFilter={toolFilter()}
        statusFilter={statusFilter()}
        onToolChange={setToolFilter}
        onStatusChange={setStatusFilter}
        tools={uniqueTools()}
        totalSessions={store.list.length}
      />

      <Show when={store.loading}>
        <div class="flex justify-center items-center py-12" role="status" aria-live="polite">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500" aria-hidden="true"></div>
          <span class="ml-3 text-gray-600">Loading sessions...</span>
          <span class="sr-only">Loading AI sessions data</span>
        </div>
      </Show>

      <Show when={store.error}>
        <div class="bg-red-50 border border-red-200 rounded-lg p-4" role="alert" aria-live="assertive">
          <div class="flex items-start">
            <div class="flex-shrink-0">
              <span class="text-red-500" aria-hidden="true">❌</span>
            </div>
            <div class="ml-3">
              <h3 class="text-sm font-medium text-red-800">Error Loading Sessions</h3>
              <div class="mt-2 text-sm text-red-700">
                {store.error}
              </div>
              <button 
                onClick={() => actions.fetchList()}
                class="mt-3 text-sm text-red-800 hover:text-red-900 font-medium focus:outline-none focus:underline"
              >
                Try Again
              </button>
            </div>
          </div>
        </div>
      </Show>

      <Show when={!store.loading && filteredSessions().length === 0}>
        <div class="text-center py-12 bg-white rounded-lg border border-gray-200">
          <div class="mx-auto max-w-md">
            <div class="text-gray-400 text-6xl mb-4" aria-hidden="true">🤖</div>
            <h3 class="text-lg font-medium text-gray-900 mb-2">
              {(toolFilter() || statusFilter()) ? 'No matching sessions' : 'No AI sessions yet'}
            </h3>
            <p class="text-gray-500 mb-4">
              {(toolFilter() || statusFilter()) 
                ? 'Try adjusting your filters to see more sessions.'
                : 'Start your first AI session using the nocodo CLI.'}
            </p>
            <Show when={toolFilter() || statusFilter()}>
              <button
                onClick={() => { setToolFilter(''); setStatusFilter(''); }}
                class="inline-flex items-center px-4 py-2 text-sm font-medium text-blue-700 bg-blue-100 border border-blue-300 rounded-md hover:bg-blue-200 focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <span class="mr-2" aria-hidden="true">🔄</span>
                Clear all filters
              </button>
            </Show>
          </div>
        </div>
      </Show>

      <Show when={!store.loading && filteredSessions().length > 0}>
        <div class="bg-white shadow-sm overflow-hidden rounded-lg border border-gray-200">
          <div class="px-4 py-3 border-b border-gray-200 bg-gray-50">
            <h2 class="text-sm font-medium text-gray-900">
              Sessions ({filteredSessions().length})
            </h2>
          </div>
          <ul 
            class="divide-y divide-gray-200"
            role="list"
            aria-label="AI Sessions list"
          >
            <For each={filteredSessions()}>
              {(session) => {
                const project = projects().find(p => p.id === session.project_id);
                return (
                  <SessionRow 
                    session={session}
                    project={project}
                    showPrompt={true}
                  />
                );
              }}
            </For>
          </ul>
        </div>
      </Show>
    </div>
  );
};

export default AiSessionsList;
