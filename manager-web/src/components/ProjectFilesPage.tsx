import { Component, Show, createSignal, onMount } from 'solid-js';
import { useNavigate, useParams } from '@solidjs/router';
import { FileInfo, Project } from '../types';
import { apiClient } from '../api';
import FileBrowser from './FileBrowser';
import FileEditor from './FileEditor';

const ProjectFilesPage: Component = () => {
  const params = useParams();
  const navigate = useNavigate();
  const [project, setProject] = createSignal<Project | null>(null);
  const [selectedFile, setSelectedFile] = createSignal<FileInfo | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  // Optionally load project details for display name, but don't block UI
  onMount(async () => {
    const projectId = params.id;
    if (!projectId) {
      navigate('/');
      return;
    }

    try {
      setLoading(true);
      const proj = await apiClient.fetchProject(projectId);
      setProject(proj);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load project');
    } finally {
      setLoading(false);
    }
  });

  const handleFileSelect = (file: FileInfo) => {
    if (!file.is_directory) {
      setSelectedFile(file);
    }
  };

  const handleEditorClose = () => {
    setSelectedFile(null);
  };

  // Render immediately; show error banner if needed
  return (
    <div class='space-y-6'>
      {/* Optional error banner */}
      <Show when={error()}>
        <div class='bg-yellow-50 text-yellow-800 border border-yellow-200 rounded-md p-3 text-sm'>
          {error()}
        </div>
      </Show>

      {/* Header */}
      <div class='flex items-center justify-between'>
        <div>
          <h1 class='text-2xl font-bold text-gray-900'>{project()?.name || params.id}</h1>
          <p class='text-sm text-gray-600 mt-1'>Project files and editor</p>
        </div>
        <button
          onClick={() => navigate('/')}
          class='px-3 py-2 text-sm bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition-colors'
        >
          ← Back to Projects
        </button>
      </div>

      <div class='grid grid-cols-1 lg:grid-cols-2 gap-6 h-screen max-h-[calc(100vh-200px)]'>
        {/* File Browser */}
        <div class='border border-gray-200 rounded-lg p-4 overflow-hidden flex flex-col'>
          <FileBrowser
            projectId={params.id}
            projectName={project()?.name}
            onFileSelect={handleFileSelect}
          />
        </div>

        {/* File Editor */}
        <div class='border border-gray-200 rounded-lg p-4 overflow-hidden flex flex-col'>
          <Show
            when={selectedFile()}
            fallback={
              <div class='flex-1 flex items-center justify-center text-gray-500'>
                <div class='text-center'>
                  <div class='text-6xl mb-4'>📄</div>
                  <p class='text-lg font-medium mb-2'>No file selected</p>
                  <p class='text-sm'>Click on a file in the browser to edit it</p>
                </div>
              </div>
            }
          >
            <FileEditor project={project()!} file={selectedFile()!} onClose={handleEditorClose} />
          </Show>
        </div>
      </div>
    </div>
  );
};

export default ProjectFilesPage;
