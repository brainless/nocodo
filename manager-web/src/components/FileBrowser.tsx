import { Component, For, Show, createEffect, createSignal } from 'solid-js';
import { FileInfo, FileListResponse } from '../types';
import { apiClient } from '../api';

interface FileBrowserProps {
  projectId: string | number;
  projectName?: string;
  onFileSelect?: (file: FileInfo) => void;
  hideDelete?: boolean; // hide actions column (delete)
}

const FileBrowser: Component<FileBrowserProps> = props => {
  const [files, setFiles] = createSignal<FileInfo[]>([]);
  const [currentPath, setCurrentPath] = createSignal<string>('');
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [selectedFile, setSelectedFile] = createSignal<FileInfo | null>(null);
  const [showCreateDialog, setShowCreateDialog] = createSignal(false);
  const [newFileName, setNewFileName] = createSignal('');
  const [newFileIsDirectory, setNewFileIsDirectory] = createSignal(false);

  // Load files for the current path
  const loadFiles = async (path: string = '') => {
    if (!props.projectId) return;

    setLoading(true);
    setError(null);

    try {
      const projectIdNum =
        typeof props.projectId === 'string' ? parseInt(props.projectId, 10) : props.projectId;

      const response: FileListResponse = await apiClient.listFiles({
        project_id: projectIdNum,
        path: path || null,
      });

      setFiles(response.files);
      setCurrentPath(response.current_path);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load files');
    } finally {
      setLoading(false);
    }
  };

  // Load root on first render and when project id changes
  createEffect(() => {
    const projectId = props.projectId;
    if (projectId) {
      loadFiles('');
    }
  });

  const navigateToDirectory = (file: FileInfo) => {
    if (file.is_directory) {
      loadFiles(file.path);
    } else {
      setSelectedFile(file);
      props.onFileSelect?.(file);
    }
  };

  const navigateUp = () => {
    const pathParts = currentPath()
      .split('/')
      .filter(part => part !== '');
    if (pathParts.length > 0) {
      pathParts.pop();
      const parentPath = pathParts.join('/');
      loadFiles(parentPath);
    }
  };

  const createFile = async () => {
    if (!props.projectId || !newFileName().trim()) return;

    try {
      const fileName = newFileName().trim();
      const filePath = currentPath() ? `${currentPath()}/${fileName}` : fileName;
      const projectIdNum =
        typeof props.projectId === 'string' ? parseInt(props.projectId, 10) : props.projectId;

      await apiClient.createFile({
        project_id: projectIdNum,
        path: filePath,
        content: newFileIsDirectory() ? null : '',
        is_directory: newFileIsDirectory(),
      });

      setShowCreateDialog(false);
      setNewFileName('');
      setNewFileIsDirectory(false);

      // Reload current directory
      loadFiles(currentPath());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create file');
    }
  };

  const deleteFile = async (file: FileInfo) => {
    if (!props.projectId) return;

    if (!confirm(`Are you sure you want to delete ${file.name}?`)) {
      return;
    }

    try {
      const projectIdNum =
        typeof props.projectId === 'string' ? parseInt(props.projectId, 10) : props.projectId;
      await apiClient.deleteFile(file.path, projectIdNum);

      // Reload current directory
      loadFiles(currentPath());

      // Clear selection if deleted file was selected
      if (selectedFile()?.path === file.path) {
        setSelectedFile(null);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete file');
    }
  };

  const formatSize = (bytes?: number) => {
    if (!bytes) return '-';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const formatDate = (timestamp?: number) => {
    if (!timestamp) return '-';
    return new Date(timestamp * 1000).toLocaleString();
  };

  return (
    <div class='space-y-4'>
      {/* Header */}
      <div class='flex items-center justify-between'>
        <div>
          <h2 class='text-lg font-medium text-gray-900'>Files</h2>
          <p class='text-sm text-gray-600'>
            Project: {props.projectName || props.projectId}
            {currentPath() && (
              <span class='ml-2 font-mono text-xs bg-gray-100 px-2 py-1 rounded'>
                {currentPath()}
              </span>
            )}
          </p>
        </div>

        <div class='flex space-x-2'>
          <button
            onClick={() => setShowCreateDialog(true)}
            class='px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors'
          >
            New
          </button>
          <button
            onClick={() => loadFiles(currentPath())}
            class='px-3 py-1 text-sm bg-gray-500 text-white rounded hover:bg-gray-600 transition-colors'
            disabled={loading()}
          >
            Refresh
          </button>
        </div>
      </div>

      {/* Error display */}
      <Show when={error()}>
        <div class='bg-red-50 border border-red-200 rounded-md p-3'>
          <p class='text-sm text-red-600'>{error()}</p>
        </div>
      </Show>

      {/* Navigation */}
      <Show when={currentPath()}>
        <div class='flex items-center space-x-2'>
          <button
            onClick={navigateUp}
            class='px-2 py-1 text-sm bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition-colors'
          >
            ← Up
          </button>
          <span class='text-sm text-gray-500'>Current: /{currentPath()}</span>
        </div>
      </Show>

      {/* Loading indicator (non-blocking) */}
      <Show when={loading()}>
        <div class='text-xs text-gray-500'>Loading files...</div>
      </Show>

      {/* File list */}
      <div class='border border-gray-200 rounded-lg overflow-hidden'>
        <Show
          when={files().length > 0}
          fallback={
            <div class='p-4 text-center text-gray-500'>
              {loading() ? 'Loading files...' : 'No files found'}
            </div>
          }
        >
          <table class='w-full'>
            <thead class='bg-gray-50'>
              <tr>
                <th class='px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase'>
                  Name
                </th>
                <th class='px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase'>
                  Size
                </th>
                <th class='px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase'>
                  Modified
                </th>
                {!props.hideDelete && (
                  <th class='px-4 py-2 text-right text-xs font-medium text-gray-500 uppercase'>
                    Actions
                  </th>
                )}
              </tr>
            </thead>
            <tbody class='divide-y divide-gray-200'>
              <For each={files()}>
                {file => (
                  <tr
                    class={`hover:bg-gray-50 ${selectedFile()?.path === file.path ? 'bg-blue-50' : ''}`}
                  >
                    <td class='px-4 py-2'>
                      <button
                        onClick={() => navigateToDirectory(file)}
                        class='flex items-center space-x-2 text-left hover:text-blue-600 transition-colors'
                      >
                        <span class='text-gray-400'>{file.is_directory ? '📁' : '📄'}</span>
                        <span class={`${file.is_directory ? 'font-medium' : ''}`}>{file.name}</span>
                      </button>
                    </td>
                    <td class='px-4 py-2 text-sm text-gray-600'>
                      {file.is_directory ? '-' : formatSize(Number(file.size ?? 0n))}
                    </td>
                    <td class='px-4 py-2 text-sm text-gray-600'>
                      {formatDate(file.modified_at ? parseInt(file.modified_at) : undefined)}
                    </td>
                    {!props.hideDelete && (
                      <td class='px-4 py-2 text-right'>
                        <button
                          onClick={() => deleteFile(file)}
                          class='text-sm text-red-600 hover:text-red-800 transition-colors'
                        >
                          Delete
                        </button>
                      </td>
                    )}
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </Show>
      </div>

      {/* Create dialog */}
      <Show when={showCreateDialog()}>
        <div class='fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50'>
          <div class='bg-white rounded-lg p-6 w-96 max-w-full mx-4'>
            <h3 class='text-lg font-medium mb-4'>Create New</h3>

            <div class='space-y-4'>
              <div>
                <label class='flex items-center space-x-2'>
                  <input
                    type='checkbox'
                    checked={newFileIsDirectory()}
                    onChange={e => setNewFileIsDirectory(e.target.checked)}
                    class='rounded'
                  />
                  <span class='text-sm'>Directory</span>
                </label>
              </div>

              <div>
                <label class='block text-sm font-medium text-gray-700 mb-1'>
                  {newFileIsDirectory() ? 'Directory' : 'File'} Name
                </label>
                <input
                  type='text'
                  value={newFileName()}
                  onInput={e => setNewFileName(e.target.value)}
                  class='w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500'
                  placeholder='Enter name...'
                  onKeyDown={e => {
                    if (e.key === 'Enter') {
                      createFile();
                    }
                  }}
                />
              </div>
            </div>

            <div class='flex justify-end space-x-2 mt-6'>
              <button
                onClick={() => {
                  setShowCreateDialog(false);
                  setNewFileName('');
                  setNewFileIsDirectory(false);
                }}
                class='px-4 py-2 text-sm text-gray-600 hover:text-gray-800 transition-colors'
              >
                Cancel
              </button>
              <button
                onClick={createFile}
                disabled={!newFileName().trim()}
                class='px-4 py-2 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors'
              >
                Create
              </button>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default FileBrowser;
