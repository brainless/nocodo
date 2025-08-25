import { Component, For, createSignal, onMount } from 'solid-js';
import { A } from '@solidjs/router';
import { Project } from '../types';
import { apiClient } from '../api';

interface ProjectListProps {
  onRefresh?: () => void;
}

const ProjectList: Component<ProjectListProps> = props => {
  const [projects, setProjects] = createSignal<Project[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  // Get WebSocket context
  // const { store: wsStore } = useWebSocket();

  const loadProjects = async () => {
    try {
      setLoading(true);
      setError(null);
      const projectList = await apiClient.fetchProjects();
      // Ensure we always have an array
      const projects = Array.isArray(projectList) ? projectList : [];
      setProjects(projects);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load projects');
      setProjects([]); // Set empty array on error to prevent map error
    } finally {
      setLoading(false);
    }
  };

  const deleteProject = async (id: string) => {
    if (!confirm('Are you sure you want to delete this project?')) {
      return;
    }

    try {
      await apiClient.deleteProject(id);
      await loadProjects();
      props.onRefresh?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete project');
    }
  };

  onMount(loadProjects);

  // Handle WebSocket messages for real-time updates - temporarily disabled for debugging
  // createEffect(() => {
  //   const message = wsStore.lastMessage;
  //   if (!message) return;
  //
  //   switch (message.type) {
  //     case 'ProjectCreated':
  //       console.log('Project created via WebSocket:', message.payload.project);
  //       // Add the new project to the list
  //       setProjects(prev => [...prev, message.payload.project]);
  //       break;
  //
  //     case 'ProjectUpdated':
  //       console.log('Project updated via WebSocket:', message.payload.project);
  //       // Update the existing project in the list
  //       setProjects(prev =>
  //         prev.map(p =>
  //           p.id === message.payload.project.id ? message.payload.project : p
  //         )
  //       );
  //       break;
  //
  //     case 'ProjectDeleted':
  //       console.log('Project deleted via WebSocket:', message.payload.project_id);
  //       // Remove the project from the list
  //       setProjects(prev =>
  //         prev.filter(p => p.id !== message.payload.project_id)
  //       );
  //       break;
  //
  //     case 'ProjectStatusChanged':
  //       console.log('Project status changed via WebSocket:', message.payload);
  //       // Update the project status
  //       setProjects(prev =>
  //         prev.map(p =>
  //           p.id === message.payload.project_id
  //             ? { ...p, status: message.payload.status }
  //             : p
  //         )
  //       );
  //       break;
  //   }
  // });

  return (
    <div class='w-full'>
      <div class='flex items-center justify-between mb-4'>
        <h2 class='text-xl font-semibold'>Projects</h2>
        <button
          onClick={loadProjects}
          class='px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600'
          disabled={loading()}
        >
          {loading() ? 'Loading...' : 'Refresh'}
        </button>
      </div>

      {error() && (
        <div class='mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded'>{error()}</div>
      )}

      {loading() ? (
        <div class='flex items-center justify-center py-8'>
          <div class='text-gray-500'>Loading projects...</div>
        </div>
      ) : projects().length === 0 ? (
        <div class='text-center py-8 text-gray-500'>
          <p>No projects found.</p>
          <p class='text-sm mt-2'>Create your first project to get started!</p>
        </div>
      ) : (
        <div class='space-y-3'>
          <For each={projects()}>
            {project => (
              <div class='p-4 border border-gray-200 rounded-lg bg-white shadow-sm'>
                <div class='flex items-start justify-between'>
                  <div>
                    <h3 class='font-medium text-lg'>{project.name}</h3>
                    <p class='text-sm text-gray-600 mt-1'>{project.path}</p>
                    <div class='flex items-center gap-4 mt-2 text-sm text-gray-500'>
                      {project.language && (
                        <span class='bg-blue-100 text-blue-800 px-2 py-1 rounded'>
                          {project.language}
                        </span>
                      )}
                      {project.framework && (
                        <span class='bg-green-100 text-green-800 px-2 py-1 rounded'>
                          {project.framework}
                        </span>
                      )}
                      <span
                        class={`px-2 py-1 rounded text-xs uppercase font-medium ${
                          project.status === 'active'
                            ? 'bg-green-100 text-green-800'
                            : project.status === 'inactive'
                              ? 'bg-gray-100 text-gray-800'
                              : 'bg-yellow-100 text-yellow-800'
                        }`}
                      >
                        {project.status}
                      </span>
                    </div>
                    <p class='text-xs text-gray-400 mt-2'>
                      Created: {new Date(project.created_at * 1000).toLocaleDateString()}
                    </p>
                  </div>
                  <div class='flex flex-col space-y-2'>
                    <A
                      href={`/projects/${project.id}/work`}
                      class='px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors text-center'
                    >
                      Open
                    </A>
                    <button
                      onClick={() => deleteProject(project.id)}
                      class='px-3 py-1 text-sm text-red-500 hover:text-red-700 border border-red-500 hover:border-red-700 rounded transition-colors'
                      title='Delete project'
                    >
                      Delete
                    </button>
                  </div>
                </div>
              </div>
            )}
          </For>
        </div>
      )}
    </div>
  );
};

export default ProjectList;
