import { Component, createSignal } from 'solid-js';
import { CreateProjectRequest } from '../types';
import { apiClient } from '../api';

interface CreateProjectFormProps {
  onProjectCreated?: () => void;
}

const CreateProjectForm: Component<CreateProjectFormProps> = (props) => {
  const [formData, setFormData] = createSignal<CreateProjectRequest>({
    name: '',
    language: '',
    framework: '',
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
        ...(data.language && { language: data.language }),
        ...(data.framework && { framework: data.framework }),
      };
      
      await apiClient.createProject(projectData);
      
      // Reset form
      setFormData({
        name: '',
        language: '',
        framework: '',
      });
      
      props.onProjectCreated?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create project');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div class="w-full max-w-md mx-auto">
      <h2 class="text-xl font-semibold mb-4">Create New Project</h2>
      
      <form onSubmit={handleSubmit} class="space-y-4">
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">
            Project Name*
          </label>
          <input
            type="text"
            value={formData().name}
            onInput={(e) => updateFormData('name', e.currentTarget.value)}
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            placeholder="my-awesome-project"
            required
          />
        </div>

        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">
            Language (Optional)
          </label>
          <select
            value={formData().language}
            onChange={(e) => updateFormData('language', e.currentTarget.value)}
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          >
            <option value="">Select language...</option>
            <option value="rust">Rust</option>
            <option value="typescript">TypeScript</option>
            <option value="python">Python</option>
            <option value="go">Go</option>
            <option value="javascript">JavaScript</option>
          </select>
        </div>

        <div>
          <label class="block text-sm font-medium text-gray-700 mb-1">
            Framework (Optional)
          </label>
          <select
            value={formData().framework}
            onChange={(e) => updateFormData('framework', e.currentTarget.value)}
            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
          >
            <option value="">Select framework...</option>
            <option value="actix-web">Actix Web</option>
            <option value="solidjs">SolidJS</option>
            <option value="fastapi">FastAPI</option>
            <option value="gin">Gin</option>
            <option value="express">Express</option>
            <option value="react">React</option>
            <option value="vue">Vue</option>
          </select>
        </div>

        {error() && (
          <div class="p-3 bg-red-100 border border-red-400 text-red-700 rounded">
            {error()}
          </div>
        )}

        <button
          type="submit"
          disabled={loading()}
          class="w-full bg-blue-500 text-white py-2 px-4 rounded-md hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {loading() ? 'Creating...' : 'Create Project'}
        </button>
      </form>
    </div>
  );
};

export default CreateProjectForm;
