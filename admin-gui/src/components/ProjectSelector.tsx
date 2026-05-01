import { For, Show, createSignal } from 'solid-js';
import { useNavigate, useLocation, useParams } from '@solidjs/router';
import { useProject } from '../contexts/ProjectContext';
import type { Project } from '../types/api';

export default function ProjectSelector() {
  const navigate = useNavigate();
  const location = useLocation();
  const params = useParams<{ projectId: string }>();
  const { projects, currentProject, setCurrentProject, isLoading, createProject } = useProject();
  const [isDropdownOpen, setIsDropdownOpen] = createSignal(false);
  const [isModalOpen, setIsModalOpen] = createSignal(false);
  const [newProjectName, setNewProjectName] = createSignal('');
  const [isCreating, setIsCreating] = createSignal(false);

  const handleCreateProject = async () => {
    const name = newProjectName().trim();
    if (!name) return;
    setIsCreating(true);
    const project = await createProject(name);
    setIsCreating(false);
    if (project) {
      setNewProjectName('');
      setIsModalOpen(false);
    }
  };

  const toggleDropdown = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDropdownOpen(!isDropdownOpen());
  };

  const closeDropdown = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDropdownOpen(false);
  };

  const selectProject = (project: Project) => {
    setCurrentProject(project);
    setIsDropdownOpen(false);
    if (String(project.id) !== params.projectId) {
      // Derive sub-route from the last path segment, build base-relative path
      const segments = location.pathname.split('/').filter(Boolean);
      const subRoute = segments[segments.length - 1];
      navigate(`/projects/${project.id}/${subRoute}`);
    }
  };

  const openCreateModal = () => {
    setIsDropdownOpen(false);
    setIsModalOpen(true);
  };

  return (
    <>
      <div class="relative">
        <button class="btn btn-ghost btn-sm gap-2" onClick={toggleDropdown}>
          <Show when={isLoading()}>
            <span class="loading loading-spinner loading-xs" />
          </Show>
          <span class="font-semibold">{currentProject()?.name ?? 'Select Project'}</span>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>
        </button>

        <Show when={isDropdownOpen()}>
          <div class="absolute top-full left-0 z-[100] menu p-2 shadow bg-base-100 rounded-box w-56 mt-2 border border-base-300">
            <div class="px-3 py-2 text-xs font-semibold text-base-content/60 uppercase tracking-wider">
              Projects
            </div>
            <For each={projects()}>
              {(project) => (
                <button
                  class={`btn btn-ghost btn-sm justify-start ${project.id === currentProject()?.id ? 'btn-active' : ''}`}
                  onClick={() => selectProject(project)}
                >
                  <span class="truncate">{project.name}</span>
                </button>
              )}
            </For>
            <div class="divider my-1" />
            <button class="btn btn-ghost btn-sm justify-start text-primary" onClick={openCreateModal}>
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-1"><path d="M5 12h14"/><path d="M12 5v14"/></svg>
              Create Project
            </button>
          </div>
        </Show>

        <Show when={isDropdownOpen()}>
          <div class="fixed inset-0 z-[99]" onClick={closeDropdown} />
        </Show>
      </div>

      <Show when={isModalOpen()}>
        <div class="modal modal-open z-[200]">
          <div class="modal-box">
            <h3 class="font-bold text-lg mb-4">Create New Project</h3>
            <div class="form-control">
              <label class="label">
                <span class="label-text">Project Name</span>
              </label>
              <input
                type="text"
                placeholder="Enter project name"
                class="input input-bordered"
                value={newProjectName()}
                onInput={(e) => setNewProjectName(e.currentTarget.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleCreateProject()}
                disabled={isCreating()}
              />
            </div>
            <div class="modal-action">
              <button
                class="btn btn-ghost"
                onClick={() => { setIsModalOpen(false); setNewProjectName(''); }}
                disabled={isCreating()}
              >
                Cancel
              </button>
              <button
                class="btn btn-primary"
                onClick={handleCreateProject}
                disabled={!newProjectName().trim() || isCreating()}
              >
                <Show when={isCreating()}>
                  <span class="loading loading-spinner loading-xs mr-2" />
                </Show>
                Create
              </button>
            </div>
          </div>
          <div
            class="modal-backdrop"
            onClick={() => { if (!isCreating()) { setIsModalOpen(false); setNewProjectName(''); } }}
          />
        </div>
      </Show>
    </>
  );
}
