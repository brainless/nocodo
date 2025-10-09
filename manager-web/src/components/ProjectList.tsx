import { Component, For, createSignal, onMount } from 'solid-js';
import { A } from '@solidjs/router';
import { Project } from '../types';
import { apiClient } from '../api';
import ProjectCard from './ProjectCard';

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

  const deleteProject = async (id: number) => {
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
      {/* Action bar without duplicate heading */}
      <div class='flex justify-between items-center mb-6'>
        <button
          onClick={loadProjects}
          class='px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors'
          disabled={loading()}
        >
          {loading() ? 'Loading...' : 'Refresh'}
        </button>

        <A
          href='/projects/create'
          class='px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors'
        >
          Create Project
        </A>
      </div>

      {error() && (
        <div class='mb-6 p-4 bg-red-50 border border-red-200 text-red-700 rounded-lg'>
          {error()}
        </div>
      )}

      {loading() ? (
        <div class='grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6'>
          {[1, 2, 3, 4, 5, 6].map(() => (
            <div class='animate-pulse'>
              <div class='bg-gray-200 rounded-lg h-48'></div>
            </div>
          ))}
        </div>
      ) : projects().length === 0 ? (
        <div class='text-center py-12'>
          <div class='mx-auto max-w-md'>
            <div class='text-gray-400 text-6xl mb-4'>📁</div>
            <h3 class='text-lg font-medium text-gray-900 mb-2'>No projects yet</h3>
            <p class='text-gray-500 mb-4'>Create your first project to get started with nocodo!</p>
            <A
              href='/projects/create'
              class='inline-flex items-center px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700'
            >
              Create Project
            </A>
          </div>
        </div>
      ) : (
        <div class='grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6'>
          <For each={projects()}>
            {project => (
              <ProjectCard project={project} showActions={true} onDelete={deleteProject} />
            )}
          </For>
        </div>
      )}
    </div>
  );
};

export default ProjectList;
