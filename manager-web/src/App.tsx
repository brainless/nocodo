import { Component, For, createSignal, onMount } from 'solid-js';
import { A, Route, Router, useParams } from '@solidjs/router';
import ProjectList from './components/ProjectList';
import CreateProjectForm from './components/CreateProjectForm';
import ProjectFilesPage from './components/ProjectFilesPage';
import AiSessionsList from './components/AiSessionsList';
import AiSessionDetail from './components/AiSessionDetail';
import Dashboard from './components/Dashboard';
import { WebSocketProvider, useWebSocketConnection } from './WebSocketProvider';
import { SessionsProvider } from './stores/sessionsStore';
import { apiClient } from './api';
import type { Project } from './types';

// Status Bar Component for the bottom of the page
const StatusBar: Component = () => {
  const { state, error } = useWebSocketConnection();

  const getStatusColor = (): string => {
    switch (state) {
      case 'connected':
        return 'bg-green-500';
      case 'connecting':
        return 'bg-yellow-500';
      case 'error':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getStatusText = (): string => {
    switch (state) {
      case 'connected':
        return 'Connected';
      case 'connecting':
        return 'Connecting...';
      case 'error':
        return 'Connection Error';
      default:
        return 'Disconnected';
    }
  };

  return (
    <div class='fixed bottom-0 left-0 right-0 bg-gray-100 border-t border-gray-200 px-4 py-2'>
      <div class='container mx-auto flex justify-between items-center'>
        <span class='text-sm text-gray-600'>nocodo Manager</span>
        <div class='flex items-center space-x-2 text-sm'>
          <div class={`w-2 h-2 rounded-full ${getStatusColor()}`}></div>
          <span class='text-gray-600'>{getStatusText()}</span>
          {error && <span class='text-red-600 text-xs'>({error})</span>}
        </div>
      </div>
    </div>
  );
};

// Top Navigation Component
const TopNavigation: Component = () => {
  const [projects, setProjects] = createSignal<Project[]>([]);
  const [isDropdownOpen, setIsDropdownOpen] = createSignal(false);
  let dropdownRef: HTMLDivElement | undefined;

  onMount(async () => {
    try {
      const projectList = await apiClient.fetchProjects();
      setProjects(projectList);
    } catch (error) {
      console.error('Failed to fetch projects:', error);
    }

    // Close dropdown when clicking outside
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
        setIsDropdownOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  });

  return (
    <nav class='bg-white border-b border-gray-200 px-4 py-3'>
      <div class='container mx-auto flex justify-between items-center'>
        <div class='flex items-center space-x-4'>
          {/* nocodo logo/home link */}
          <A href='/' class='text-xl font-bold text-gray-900 hover:text-blue-600'>
            nocodo
          </A>

          {/* Project dropdown */}
          <div class='relative' ref={dropdownRef}>
            <button
              class='flex items-center space-x-1 px-3 py-2 text-sm font-medium text-gray-700 hover:text-gray-900 hover:bg-muted rounded-md border border-border'
              onClick={() => setIsDropdownOpen(!isDropdownOpen())}
            >
              <span>Projects</span>
              <svg class='w-4 h-4' fill='none' stroke='currentColor' viewBox='0 0 24 24'>
                <path
                  stroke-linecap='round'
                  stroke-linejoin='round'
                  stroke-width={2}
                  d='M19 9l-7 7-7-7'
                />
              </svg>
            </button>

            {isDropdownOpen() && (
              <div class='absolute left-0 mt-2 w-64 bg-white rounded-md shadow-lg border border-gray-200 z-10'>
                <div class='py-1'>
                  <A href='/projects' class='block px-4 py-2 text-sm text-gray-700 hover:bg-muted'>
                    All Projects
                  </A>
                  <A
                    href='/projects/create'
                    class='block px-4 py-2 text-sm text-gray-700 hover:bg-muted'
                  >
                    Create New Project
                  </A>
                  {projects().length > 0 && (
                    <>
                      <div class='border-t border-gray-200 my-1'></div>
                      <For each={projects()}>
                        {project => (
                          <A
                            href={`/projects/${project.id}/work`}
                            class='block px-4 py-2 text-sm text-gray-700 hover:bg-muted'
                          >
                            <div class='font-medium'>{project.name}</div>
                            <div class='text-xs text-gray-500'>{project.language || 'Unknown'}</div>
                          </A>
                        )}
                      </For>
                    </>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Right side - Sessions link */}
        <div class='flex items-center'>
          <A
            href='/ai/sessions'
            class='px-3 py-2 text-sm font-medium text-gray-700 hover:text-gray-900 hover:bg-muted rounded-md border border-border'
          >
            Sessions
          </A>
        </div>
      </div>
    </nav>
  );
};

// Layout component with page heading support outside white content box
interface LayoutProps {
  children: any;
  title?: string;
  subtitle?: string;
  noBox?: boolean; // Skip the outer white content box
}

const Layout: Component<LayoutProps> = props => {
  return (
    <div class='min-h-screen bg-background pb-12'>
      {/* pb-12 for status bar space */}
      <TopNavigation />
      <main class='container mx-auto px-4 py-8'>
        {/* Page heading outside the white content box */}
        {(props.title || props.subtitle) && (
          <div class='mb-8'>
            {props.title && <h1 class='text-3xl font-bold text-gray-900 mb-2'>{props.title}</h1>}
            {props.subtitle && <p class='text-gray-600'>{props.subtitle}</p>}
          </div>
        )}

        {/* Conditional white content box */}
        {props.noBox ? (
          props.children
        ) : (
          <div class='bg-white rounded-lg shadow-sm border border-gray-200 p-6'>
            {props.children}
          </div>
        )}
      </main>
      <StatusBar />
    </div>
  );
};

// Dashboard Page
const DashboardPage: Component = () => {
  return (
    <Layout title='Welcome back!' subtitle='Manage your AI coding projects and sessions' noBox>
      <Dashboard />
    </Layout>
  );
};

// Projects Page
const ProjectsPage: Component = () => {
  return (
    <Layout title='Projects' subtitle='Manage your development projects' noBox>
      <ProjectList />
    </Layout>
  );
};

// Create Project Page
const CreateProjectPage: Component = () => {
  return (
    <Layout title='Create New Project' subtitle='Set up a new development project'>
      <CreateProjectForm />
    </Layout>
  );
};

// Files Page - now uses Layout for consistency
const FilesPageWrapper: Component = () => {
  return (
    <Layout>
      <ProjectFilesPage />
    </Layout>
  );
};

// Project Details Page
import ProjectDetails from './components/ProjectDetails';
const ProjectDetailsWrapper: Component = () => {
  const params = useParams();
  const [project, setProject] = createSignal<Project | null>(null);

  // Load project data to get the title
  onMount(async () => {
    try {
      const projectId = (params as any).id;
      if (projectId) {
        const details = await apiClient.fetchProjectDetails(projectId);
        setProject(details.project);
      }
    } catch (e) {
      console.error('Failed to load project for title:', e);
    }
  });

  return (
    <Layout title={project()?.name || 'Project'} subtitle='Project Dashboard'>
      <ProjectDetails />
    </Layout>
  );
};

// AI Sessions Pages
const AiSessionsPage: Component = () => {
  return (
    <Layout title='AI Sessions' subtitle='View and manage your AI coding sessions' noBox>
      <AiSessionsList />
    </Layout>
  );
};

const AiSessionDetailPage: Component = () => {
  return (
    <Layout title='AI Session Details' subtitle='View session information and live output' noBox>
      <AiSessionDetail />
    </Layout>
  );
};

// Root App Component - defines the routes
const App: Component = () => {
  return (
    <WebSocketProvider>
      <SessionsProvider>
        <Router>
          <Route path='/' component={DashboardPage} />
          <Route path='/projects' component={ProjectsPage} />
          <Route path='/projects/create' component={CreateProjectPage} />
          <Route path='/projects/:id' component={ProjectDetailsWrapper} />
          <Route path='/projects/:id/:tab' component={ProjectDetailsWrapper} />
          <Route path='/projects/:id/files' component={FilesPageWrapper} />
          <Route path='/ai/sessions' component={AiSessionsPage} />
          <Route path='/ai/sessions/:id' component={AiSessionDetailPage} />
        </Router>
      </SessionsProvider>
    </WebSocketProvider>
  );
};

export default App;
