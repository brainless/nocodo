import { Component, createSignal } from 'solid-js';
import { AddExistingProjectRequest, CreateProjectRequest } from '../types';
import { apiClient } from '../api';

interface CreateProjectFormProps {
  onProjectCreated?: () => void;
}

const CreateProjectForm: Component<CreateProjectFormProps> = props => {
  const [formData, setFormData] = createSignal<CreateProjectRequest>({
    name: '',
    path: null,
    language: null,
    framework: null,
    template: null,
  });
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const updateFormData = (field: keyof CreateProjectRequest, value: string) => {
    setFormData(prev => ({ ...prev, [field]: value }));
  };

  const handleSubmit = async (e: Event) => {
    e.preventDefault();

    const data = formData();
    if (!data.name.trim()) {
      setError('Project name is required');
      return;
    }

    try {
      setLoading(true);
      setError(null);

      const projectData: CreateProjectRequest = {
        name: data.name.trim(),
        path: data.path && data.path.trim() ? data.path.trim() : null,
        language: null, // manager will detect
        framework: null,
        template: null,
      };

      await apiClient.createProject(projectData);

      // Reset form
      setFormData({
        name: '',
        path: null,
        language: null,
        framework: null,
        template: null,
      });

      props.onProjectCreated?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create project');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class='w-full max-w-md mx-auto'>
      <h2 class='text-xl font-semibold mb-4'>Create New Project</h2>

      <form onSubmit={handleSubmit} class='space-y-4'>
        <div>
          <label class='block text-sm font-medium text-gray-700 mb-1'>Project Name*</label>
          <input
            type='text'
            value={formData().name}
            onInput={e => updateFormData('name', e.currentTarget.value)}
            class='w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
            placeholder='my-awesome-project'
            required
          />
        </div>

        <div>
          <label class='block text-sm font-medium text-gray-700 mb-1'>
            Project Path (Full path)
          </label>
          <input
            type='text'
            value={formData().path || ''}
            onInput={e => updateFormData('path', e.currentTarget.value)}
            class='w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
            placeholder='/home/me/Projects/my-awesome-project'
          />
          <p class='text-xs text-gray-500 mt-1'>
            Leave blank to let manager choose a default path.
          </p>
        </div>

        {error() && (
          <div class='p-3 bg-red-100 border border-red-400 text-red-700 rounded'>{error()}</div>
        )}

        <button
          type='submit'
          disabled={loading()}
          class='w-full bg-blue-500 text-white py-2 px-4 rounded-md hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed'
        >
          {loading() ? 'Creating...' : 'Create Project'}
        </button>
      </form>
    </div>
  );
};

// Add Existing Project Form Component
interface AddExistingProjectFormProps {
  onProjectAdded?: () => void;
}

export const AddExistingProjectForm: Component<AddExistingProjectFormProps> = props => {
  const [formData, setFormData] = createSignal<AddExistingProjectRequest>({
    name: '',
    path: '',
    language: null,
    framework: null,
  });
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const updateFormData = (field: keyof AddExistingProjectRequest, value: string) => {
    setFormData(prev => ({ ...prev, [field]: value }));
  };

  const handleSubmit = async (e: Event) => {
    e.preventDefault();

    const data = formData();
    if (!data.name.trim()) {
      setError('Project name is required');
      return;
    }

    if (!data.path.trim()) {
      setError('Project path is required');
      return;
    }

    try {
      setLoading(true);
      setError(null);

      const projectData: AddExistingProjectRequest = {
        name: data.name.trim(),
        path: data.path.trim(),
        language: null, // manager will detect
        framework: null,
      };

      await apiClient.addExistingProject(projectData);

      // Reset form
      setFormData({
        name: '',
        path: '',
        language: null,
        framework: null,
      });

      props.onProjectAdded?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add existing project');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class='w-full max-w-md mx-auto'>
      <h2 class='text-xl font-semibold mb-4'>Add Existing Project</h2>

      <form onSubmit={handleSubmit} class='space-y-4'>
        <div>
          <label class='block text-sm font-medium text-gray-700 mb-1'>Project Name*</label>
          <input
            type='text'
            value={formData().name}
            onInput={e => updateFormData('name', e.currentTarget.value)}
            class='w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
            placeholder='my-existing-project'
            required
          />
        </div>

        <div>
          <label class='block text-sm font-medium text-gray-700 mb-1'>
            Project Path (Full path)*
          </label>
          <input
            type='text'
            value={formData().path}
            onInput={e => updateFormData('path', e.currentTarget.value)}
            class='w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent'
            placeholder='/home/me/Projects/my-existing-project'
            required
          />
          <p class='text-xs text-gray-500 mt-1'>Full path to the existing project directory.</p>
        </div>

        {error() && (
          <div class='p-3 bg-red-100 border border-red-400 text-red-700 rounded'>{error()}</div>
        )}

        <button
          type='submit'
          disabled={loading()}
          class='w-full bg-blue-500 text-white py-2 px-4 rounded-md hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed'
        >
          {loading() ? 'Adding...' : 'Add Existing Project'}
        </button>
      </form>
    </div>
  );
};

export default CreateProjectForm;
