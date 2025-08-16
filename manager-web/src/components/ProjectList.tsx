import { Component, createSignal, onMount } from 'solid-js';
import { Project } from '../types';
import { apiClient } from '../api';

interface ProjectListProps {
  onRefresh?: () => void;
}

const ProjectList: Component<ProjectListProps> = (props) => {
  const [projects, setProjects] = createSignal<Project[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const loadProjects = async () => {
    try {
      setLoading(true);
      setError(null);
      const projectList = await apiClient.fetchProjects();
      setProjects(projectList);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load projects');
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

  return (
    <div class="w-full">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-xl font-semibold">Projects</h2>
        <button 
          onClick={loadProjects}
          class="px-3 py-1 bg-blue-500 text-white rounded hover:bg-blue-600"
          disabled={loading()}
        >
          {loading() ? 'Loading...' : 'Refresh'}
        </button>
      </div>

      {error() && (
        <div class="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
          {error()}
        </div>
      )}

      {loading() ? (
        <div class="flex items-center justify-center py-8">
          <div class="text-gray-500">Loading projects...</div>
        </div>
      ) : projects().length === 0 ? (
        <div class="text-center py-8 text-gray-500">
          <p>No projects found.</p>
          <p class="text-sm mt-2">Create your first project to get started!</p>
        </div>
      ) : (
        <div class="space-y-3">
          {projects().map((project) => (
            <div class="p-4 border border-gray-200 rounded-lg bg-white shadow-sm">
              <div class="flex items-start justify-between">
                <div>
                  <h3 class="font-medium text-lg">{project.name}</h3>
                  <p class="text-sm text-gray-600 mt-1">{project.path}</p>
                  <div class="flex items-center gap-4 mt-2 text-sm text-gray-500">
                    {project.language && (
                      <span class="bg-blue-100 text-blue-800 px-2 py-1 rounded">
                        {project.language}
                      </span>
                    )}
                    {project.framework && (
                      <span class="bg-green-100 text-green-800 px-2 py-1 rounded">
                        {project.framework}
                      </span>
                    )}
                    <span class={`px-2 py-1 rounded text-xs uppercase font-medium ${
                      project.status === 'active' ? 'bg-green-100 text-green-800' : 
                      project.status === 'inactive' ? 'bg-gray-100 text-gray-800' :
                      'bg-yellow-100 text-yellow-800'
                    }`}>
                      {project.status}
                    </span>
                  </div>
                  <p class="text-xs text-gray-400 mt-2">
                    Created: {new Date(project.created_at * 1000).toLocaleDateString()}
                  </p>
                </div>
                <button
                  onClick={() => deleteProject(project.id)}
                  class="text-red-500 hover:text-red-700 text-sm"
                  title="Delete project"
                >
                  Delete
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default ProjectList;
