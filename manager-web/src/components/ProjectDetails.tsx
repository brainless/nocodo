import { A, useNavigate, useParams } from '@solidjs/router';
import { Component, For, Show, createSignal, onMount } from 'solid-js';
import { ExtendedAiSession, Project } from '../types';
import { apiClient } from '../api';
import FileBrowser from './FileBrowser';
import FileEditor from './FileEditor';
import AiSessionCard from './AiSessionCard';

interface ProjectComponentInfo {
  id: string;
  project_id: string;
  name: string;
  path: string; // relative
  language: string;
  framework?: string | null;
  created_at: number;
}

interface ProjectTechnology {
  language: string;
  framework?: string | null;
  file_count: number;
  confidence: number;
}

interface ProjectDetectionResult {
  primary_language: string;
  technologies: ProjectTechnology[];
  build_tools: string[];
  package_managers: string[];
  deployment_configs: string[];
}

const Tabs = ['work', 'code', 'project', 'automation', 'about'] as const;

type TabKey = (typeof Tabs)[number];

const TabLabel: Record<TabKey, string> = {
  work: 'Work',
  code: 'Code',
  project: 'Project Management',
  automation: 'Automation',
  about: 'About',
};

const ProjectDetails: Component = () => {
  const params = useParams();
  const navigate = useNavigate();

  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  // current tab from URL
  const currentTab = () => {
    const raw = (params as unknown as { tab?: string }).tab || 'work';
    return (['work', 'code', 'project', 'automation', 'about'] as const).includes(raw as any)
      ? (raw as TabKey)
      : 'work';
  };

  const [project, setProject] = createSignal<Project | null>(null);
  const [components, setComponents] = createSignal<ProjectComponentInfo[]>([]);
  const [sessions, setSessions] = createSignal<ExtendedAiSession[]>([]);
  const [selectedFile, setSelectedFile] = createSignal<any>(null);

  const projectId = () => params.id;

  const loadData = async () => {
    try {
      setLoading(true);
      setError(null);

      const pid = projectId();
      if (!pid) {
        navigate('/projects');
        return;
      }

      // Fetch project details + components
      const details = await apiClient.fetchProjectDetails(pid);
      setProject(details.project);
      setComponents(details.components as ProjectComponentInfo[]);

      // Fetch sessions and filter by project
      const all = await apiClient.listSessions();
      setSessions(Array.isArray(all) ? all.filter(s => s.project_id === pid) : []);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load project details');
    } finally {
      setLoading(false);
    }
  };

  onMount(loadData);

  const isActive = (k: TabKey) =>
    currentTab() === k
      ? 'border-blue-600 text-blue-600'
      : 'border-transparent text-gray-600 hover:text-gray-800';

  return (
    <div class='space-y-6'>
      {/* Tabs - outside white box */}
      <div class='border-b border-gray-200'>
        <nav class='-mb-px flex space-x-6' aria-label='Tabs'>
          <For each={Tabs as unknown as TabKey[]}>
            {key => (
              <A
                href={`/projects/${projectId()}/${key}`}
                class={`whitespace-nowrap border-b-2 px-3 py-2 text-sm font-medium ${isActive(key)}`}
              >
                {TabLabel[key]}
              </A>
            )}
          </For>
        </nav>
      </div>

      {/* Content */}
      <div>
        <Show when={!loading()} fallback={<div class='text-gray-500'>Loading...</div>}>
          <Show
            when={!error()}
            fallback={
              <div class='p-3 bg-red-50 border border-red-200 text-red-700 text-sm rounded'>
                {error()}
              </div>
            }
          >
            {/* Work Tab */}
            <Show when={currentTab() === 'work'}>
              <div class='space-y-6'>
                <Show
                  when={sessions().length > 0}
                  fallback={<div class='text-gray-500'>No sessions found for this project.</div>}
                >
                  <div class='grid grid-cols-1 gap-6'>
                    <For each={sessions()}>
                      {s => <AiSessionCard session={s} project={project()} showPrompt={true} />}
                    </For>
                  </div>
                </Show>
              </div>
            </Show>

            {/* Code Tab */}
            <Show when={currentTab() === 'code'}>
              <div class='grid grid-cols-1 lg:grid-cols-[320px_1fr] gap-4'>
                {/* Narrow file list */}
                <div class='border border-gray-200 rounded-lg p-2 w-full max-w-[320px]'>
                  <FileBrowser
                    projectId={projectId()}
                    projectName={project()?.name}
                    hideDelete={true}
                  />
                </div>

                {/* File content viewer/editor */}
                <div class='border border-gray-200 rounded-lg p-2 min-h-[400px]'>
                  <Show
                    when={selectedFile()}
                    fallback={
                      <div class='text-gray-500 flex items-center justify-center h-full'>
                        Select a file to view
                      </div>
                    }
                  >
                    <FileEditor
                      project={project()!}
                      file={selectedFile()!}
                      onClose={() => setSelectedFile(null)}
                    />
                  </Show>
                </div>
              </div>
            </Show>

            {/* Project Management Tab */}
            <Show when={currentTab() === 'project'}>
              <div class='text-gray-600 text-sm'>Coming soon</div>
            </Show>

            {/* Automation Tab */}
            <Show when={currentTab() === 'automation'}>
              <div class='text-gray-600 text-sm'>Coming soon</div>
            </Show>

            {/* About Tab */}
            <Show when={currentTab() === 'about'}>
              <div class='space-y-4'>
                <div class='bg-white border border-gray-200 rounded p-4'>
                  <h3 class='text-lg font-semibold mb-2'>Project</h3>
                  <div class='text-sm text-gray-700'>
                    <div>
                      <span class='font-medium'>Name:</span> {project()?.name}
                    </div>
                    <div>
                      <span class='font-medium'>Path:</span> {project()?.path}
                    </div>
                    <div>
                      <span class='font-medium'>Language:</span> {project()?.language ?? 'Unknown'}
                    </div>
                    <div>
                      <span class='font-medium'>Framework:</span> {project()?.framework ?? 'N/A'}
                    </div>
                    <div>
                      <span class='font-medium'>Status:</span> {project()?.status}
                    </div>
                  </div>
                </div>

                <Show when={project()?.technologies}>
                  <div class='bg-white border border-gray-200 rounded p-4'>
                    <h3 class='text-lg font-semibold mb-3'>Technologies Detected</h3>
                    <div class='space-y-2'>
                      <For
                        each={(() => {
                          try {
                            const detection = JSON.parse(
                              project()!.technologies!
                            ) as ProjectDetectionResult;
                            return detection.technologies.sort(
                              (a, b) => b.file_count - a.file_count
                            );
                          } catch (e) {
                            console.error('Failed to parse technologies:', e);
                            return [];
                          }
                        })()}
                      >
                        {tech => (
                          <div class='p-3 border border-gray-200 rounded flex items-center justify-between'>
                            <div>
                              <div class='font-medium text-gray-900'>
                                {tech.language}
                                {tech.framework && (
                                  <span class='ml-2 text-sm text-gray-600'>• {tech.framework}</span>
                                )}
                              </div>
                              <div class='text-xs text-gray-500'>
                                {tech.file_count} files • {Math.round(tech.confidence * 100)}%
                                confidence
                              </div>
                            </div>
                          </div>
                        )}
                      </For>
                    </div>
                  </div>
                </Show>

                {/* Additional project metadata sections */}
                <Show when={project()?.technologies}>
                  <div class='grid grid-cols-1 md:grid-cols-3 gap-4'>
                    {/* Build Tools */}
                    <div class='bg-white border border-gray-200 rounded p-4'>
                      <h3 class='text-lg font-semibold mb-3'>Build Tools</h3>
                      <div class='space-y-2'>
                        <For
                          each={(() => {
                            try {
                              const detection = JSON.parse(
                                project()!.technologies!
                              ) as ProjectDetectionResult;
                              return detection.build_tools || [];
                            } catch (e) {
                              return [];
                            }
                          })()}
                        >
                          {tool => (
                            <div class='p-2 bg-gray-50 rounded text-sm text-gray-700'>{tool}</div>
                          )}
                        </For>
                        <Show
                          when={(() => {
                            try {
                              const detection = JSON.parse(
                                project()!.technologies!
                              ) as ProjectDetectionResult;
                              return !detection.build_tools || detection.build_tools.length === 0;
                            } catch (e) {
                              return true;
                            }
                          })()}
                        >
                          <div class='text-sm text-gray-500'>No build tools detected</div>
                        </Show>
                      </div>
                    </div>

                    {/* Package Managers */}
                    <div class='bg-white border border-gray-200 rounded p-4'>
                      <h3 class='text-lg font-semibold mb-3'>Package Managers</h3>
                      <div class='space-y-2'>
                        <For
                          each={(() => {
                            try {
                              const detection = JSON.parse(
                                project()!.technologies!
                              ) as ProjectDetectionResult;
                              return detection.package_managers || [];
                            } catch (e) {
                              return [];
                            }
                          })()}
                        >
                          {manager => (
                            <div class='p-2 bg-gray-50 rounded text-sm text-gray-700'>
                              {manager}
                            </div>
                          )}
                        </For>
                        <Show
                          when={(() => {
                            try {
                              const detection = JSON.parse(
                                project()!.technologies!
                              ) as ProjectDetectionResult;
                              return (
                                !detection.package_managers ||
                                detection.package_managers.length === 0
                              );
                            } catch (e) {
                              return true;
                            }
                          })()}
                        >
                          <div class='text-sm text-gray-500'>No package managers detected</div>
                        </Show>
                      </div>
                    </div>

                    {/* Deployment Configs */}
                    <div class='bg-white border border-gray-200 rounded p-4'>
                      <h3 class='text-lg font-semibold mb-3'>Deployment Configs</h3>
                      <div class='space-y-2'>
                        <For
                          each={(() => {
                            try {
                              const detection = JSON.parse(
                                project()!.technologies!
                              ) as ProjectDetectionResult;
                              return detection.deployment_configs || [];
                            } catch (e) {
                              return [];
                            }
                          })()}
                        >
                          {config => (
                            <div class='p-2 bg-gray-50 rounded text-sm text-gray-700'>{config}</div>
                          )}
                        </For>
                        <Show
                          when={(() => {
                            try {
                              const detection = JSON.parse(
                                project()!.technologies!
                              ) as ProjectDetectionResult;
                              return (
                                !detection.deployment_configs ||
                                detection.deployment_configs.length === 0
                              );
                            } catch (e) {
                              return true;
                            }
                          })()}
                        >
                          <div class='text-sm text-gray-500'>No deployment configs detected</div>
                        </Show>
                      </div>
                    </div>
                  </div>
                </Show>

                <div class='bg-white border border-gray-200 rounded p-4'>
                  <h3 class='text-lg font-semibold mb-3'>Components</h3>
                  <Show
                    when={components().length > 0}
                    fallback={<div class='text-sm text-gray-600'>No components detected</div>}
                  >
                    <div class='space-y-2'>
                      <For each={components()}>
                        {c => (
                          <div class='p-3 border border-gray-200 rounded'>
                            <div class='flex items-center justify-between'>
                              <div class='font-medium text-gray-900'>{c.name}</div>
                              <div class='text-xs text-gray-500'>
                                {c.language}
                                {c.framework ? ` • ${c.framework}` : ''}
                              </div>
                            </div>
                            <div class='text-xs text-gray-500 mt-1'>/{c.path}</div>
                          </div>
                        )}
                      </For>
                    </div>
                  </Show>
                </div>
              </div>
            </Show>
          </Show>
        </Show>
      </div>
    </div>
  );
};

export default ProjectDetails;
