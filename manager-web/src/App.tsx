import { Component } from 'solid-js';
import { A, Route } from '@solidjs/router';
import ProjectList from './components/ProjectList';
import CreateProjectForm from './components/CreateProjectForm';
import ProjectFilesPage from './components/ProjectFilesPage';
import AiSessionsList from './components/AiSessionsList';
import AiSessionDetail from './components/AiSessionDetail';
import { WebSocketProvider, useWebSocketConnection } from './WebSocketProvider';
import { SessionsProvider } from './stores/sessionsStore';

// Connection Status Component
const ConnectionStatus: Component = () => {
  const { state, error } = useWebSocketConnection();
  
  const getStatusColor = () => {
    switch (state) {
      case 'connected': return 'bg-green-500';
      case 'connecting': return 'bg-yellow-500';
      case 'error': return 'bg-red-500';
      default: return 'bg-gray-500';
    }
  };
  
  const getStatusText = () => {
    switch (state) {
      case 'connected': return 'Connected';
      case 'connecting': return 'Connecting...';
      case 'error': return 'Connection Error';
      default: return 'Disconnected';
    }
  };
  
  return (
    <div class="flex items-center space-x-2 text-sm">
      <div class={`w-2 h-2 rounded-full ${getStatusColor()}`}></div>
      <span class="text-gray-600">{getStatusText()}</span>
      {error && (
        <span class="text-red-600 text-xs">({error})</span>
      )}
    </div>
  );
};

// Layout component with navigation (shared across all routes)
const Layout: Component<{ children: any }> = (props) => {
  return (
    <WebSocketProvider>
      <SessionsProvider>
        <div class="min-h-screen bg-gray-50">
          <div class="container mx-auto px-4 py-8">
          <header class="mb-8">
            <div class="flex justify-between items-start">
              <div>
                <h1 class="text-3xl font-bold text-gray-900 mb-2">nocodo Manager</h1>
                <p class="text-gray-600">AI-assisted development environment</p>
              </div>
              <ConnectionStatus />
            </div>
          </header>

          <nav class="mb-8">
            <div class="flex space-x-4">
              <A
                href="/"
                class="px-4 py-2 rounded-md font-medium transition-colors"
                activeClass="bg-blue-500 text-white"
                inactiveClass="bg-white text-gray-700 hover:bg-gray-50 border border-gray-300"
                end
              >
                Projects
              </A>
              <A
                href="/projects/create"
                class="px-4 py-2 rounded-md font-medium transition-colors"
                activeClass="bg-blue-500 text-white"
                inactiveClass="bg-white text-gray-700 hover:bg-gray-50 border border-gray-300"
              >
                Create Project
              </A>
              <A
                href="/ai/sessions"
                class="px-4 py-2 rounded-md font-medium transition-colors"
                activeClass="bg-blue-500 text-white"
                inactiveClass="bg-white text-gray-700 hover:bg-gray-50 border border-gray-300"
              >
                AI Sessions
              </A>
            </div>
          </nav>

          <main class="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
            {props.children}
          </main>

          <footer class="mt-8 text-center text-sm text-gray-500">
            <p>nocodo Manager - Minimal Web Interface</p>
          </footer>
          </div>
        </div>
      </SessionsProvider>
    </WebSocketProvider>
  );
};

// Projects Page
const ProjectsPage: Component = () => {
  return (
    <Layout>
      <ProjectList />
    </Layout>
  );
};

// Create Project Page
const CreateProjectPage: Component = () => {
  return (
    <Layout>
      <CreateProjectForm />
    </Layout>
  );
};

// Files Page (without layout since it has its own)
const FilesPageWrapper: Component = () => {
  return (
    <WebSocketProvider>
      <div class="min-h-screen bg-gray-50">
        <div class="container mx-auto px-4 py-8">
          <ProjectFilesPage />
        </div>
      </div>
    </WebSocketProvider>
  );
};

// AI Sessions Pages
const AiSessionsPage: Component = () => {
  return (
    <Layout>
      <AiSessionsList />
    </Layout>
  );
};

const AiSessionDetailPage: Component = () => {
  return (
    <Layout>
      <AiSessionDetail />
    </Layout>
  );
};

// Root App Component - defines the routes
const App: Component = () => {
  return (
    <>
      <Route path="/" component={ProjectsPage} />
      <Route path="/projects/create" component={CreateProjectPage} />
      <Route path="/projects/:id/files" component={FilesPageWrapper} />
      <Route path="/ai/sessions" component={AiSessionsPage} />
      <Route path="/ai/sessions/:id" component={AiSessionDetailPage} />
    </>
  );
};

export default App;
