import { Component, For, createSignal, onMount } from 'solid-js';
import { A } from '@solidjs/router';
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

      {/* Quick actions */}
      <div class='bg-white rounded-lg shadow-sm border border-gray-200 p-6'>
        <h3 class='text-lg font-semibold text-gray-900 mb-4'>Quick Actions</h3>
        <div class='flex flex-wrap gap-3'>
          <A
            href='/projects/create'
            class='inline-flex items-center px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700'
          >
            Create Project
          </A>
          <A
            href='/projects'
            class='inline-flex items-center px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50'
          >
            Browse Projects
          </A>
          <A
            href='/ai/sessions'
            class='inline-flex items-center px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50'
          >
            View AI Sessions
          </A>
        </div>
      </div>
    </div>
  );
};

export default Dashboard;
