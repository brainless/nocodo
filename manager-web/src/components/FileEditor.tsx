import { Component, createSignal, createEffect, Show } from 'solid-js';
import { FileInfo, FileContentResponse, Project } from '../types';
import { apiClient } from '../api';

interface FileEditorProps {
  project: Project;
  file: FileInfo;
  onClose: () => void;
}

const FileEditor: Component<FileEditorProps> = (props) => {
  const [content, setContent] = createSignal<string>('');
  const [originalContent, setOriginalContent] = createSignal<string>('');
  const [loading, setLoading] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [hasChanges, setHasChanges] = createSignal(false);

  // Load file content when file changes
  createEffect(() => {
    if (props.file && !props.file.is_directory) {
      loadContent();
    }
  });

  // Track changes
  createEffect(() => {
    setHasChanges(content() !== originalContent());
  });

  const loadContent = async () => {
    if (!props.project || !props.file || props.file.is_directory) return;

    setLoading(true);
    setError(null);

    try {
      const response: FileContentResponse = await apiClient.getFileContent(
        props.file.path,
        props.project.id
      );

      setContent(response.content);
      setOriginalContent(response.content);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load file content');
    } finally {
      setLoading(false);
    }
  };

  const saveFile = async () => {
    if (!props.project || !props.file || props.file.is_directory) return;

    setSaving(true);
    setError(null);

    try {
      await apiClient.updateFile(props.file.path, {
        project_id: props.project.id,
        content: content(),
      });

      setOriginalContent(content());
      setHasChanges(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save file');
    } finally {
      setSaving(false);
    }
  };

  const handleClose = () => {
    if (hasChanges()) {
      if (confirm('You have unsaved changes. Are you sure you want to close?')) {
        props.onClose();
      }
    } else {
      props.onClose();
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    // Ctrl+S to save
    if (e.ctrlKey && e.key === 's') {
      e.preventDefault();
      saveFile();
    }
  };

  // Show error if trying to edit a directory
  if (props.file?.is_directory) {
    return (
      <div class="space-y-4">
        <div class="flex items-center justify-between">
          <h2 class="text-lg font-medium text-gray-900">
            Cannot Edit Directory
          </h2>
          <button
            onClick={props.onClose}
            class="text-gray-500 hover:text-gray-700 transition-colors"
          >
            ✕
          </button>
        </div>
        <div class="bg-yellow-50 border border-yellow-200 rounded-md p-3">
          <p class="text-sm text-yellow-700">
            Cannot edit directory "{props.file.name}". Please select a file to edit.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div class="space-y-4 h-full flex flex-col">
      {/* Header */}
      <div class="flex items-center justify-between flex-shrink-0">
        <div>
          <h2 class="text-lg font-medium text-gray-900">
            Edit File
          </h2>
          <p class="text-sm text-gray-600">
            {props.file.name}
            {hasChanges() && <span class="ml-2 text-orange-600">• Modified</span>}
          </p>
        </div>
        
        <div class="flex items-center space-x-2">
          <button
            onClick={saveFile}
            disabled={!hasChanges() || saving()}
            class={`px-3 py-1 text-sm rounded transition-colors ${
              hasChanges() && !saving()
                ? 'bg-green-500 text-white hover:bg-green-600'
                : 'bg-gray-300 text-gray-500 cursor-not-allowed'
            }`}
          >
            {saving() ? 'Saving...' : 'Save'}
          </button>
          <button
            onClick={handleClose}
            class="text-gray-500 hover:text-gray-700 transition-colors text-lg"
          >
            ✕
          </button>
        </div>
      </div>

      {/* Error display */}
      <Show when={error()}>
        <div class="bg-red-50 border border-red-200 rounded-md p-3 flex-shrink-0">
          <p class="text-sm text-red-600">{error()}</p>
        </div>
      </Show>

      {/* Content editor */}
      <div class="flex-1 flex flex-col">
        <Show 
          when={!loading()} 
          fallback={
            <div class="flex-1 flex items-center justify-center">
              <div class="text-center text-gray-500">
                <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900 mx-auto mb-2"></div>
                Loading file content...
              </div>
            </div>
          }
        >
          <div class="flex-1 flex flex-col">
            <div class="mb-2 flex items-center justify-between text-xs text-gray-500">
              <span>Content</span>
              <div class="flex items-center space-x-4">
                <span>Lines: {content().split('\n').length}</span>
                <span>Characters: {content().length}</span>
                <span class="text-gray-400">Ctrl+S to save</span>
              </div>
            </div>
            
            <textarea
              value={content()}
              onInput={(e) => setContent(e.target.value)}
              onKeyDown={handleKeyDown}
              class="flex-1 w-full p-4 border border-gray-300 rounded-md font-mono text-sm resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
              placeholder="File content..."
              spellcheck={false}
            />
          </div>
        </Show>
      </div>

      {/* Footer with keyboard shortcuts */}
      <div class="flex-shrink-0 text-xs text-gray-500 border-t pt-2">
        <div class="flex justify-between items-center">
          <div>
            <span class="font-medium">Shortcuts:</span>
            <span class="ml-2">Ctrl+S: Save</span>
          </div>
          <div>
            File: {props.file.path}
          </div>
        </div>
      </div>
    </div>
  );
};

export default FileEditor;
