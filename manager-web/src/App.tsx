import { Component, createSignal } from 'solid-js';
import ProjectList from './components/ProjectList';
import CreateProjectForm from './components/CreateProjectForm';

const App: Component = () => {
  const [activeView, setActiveView] = createSignal<'projects' | 'create'>('projects');
  const [refreshKey, setRefreshKey] = createSignal(0);

  const handleProjectCreated = () => {
    setActiveView('projects');
    setRefreshKey(prev => prev + 1);
  };

  const handleRefresh = () => {
    setRefreshKey(prev => prev + 1);
  };

  return (
    <div class="min-h-screen bg-gray-50">
      <div class="container mx-auto px-4 py-8">
        <header class="mb-8">
          <h1 class="text-3xl font-bold text-gray-900 mb-2">nocodo Manager</h1>
          <p class="text-gray-600">AI-assisted development environment</p>
        </header>

        <nav class="mb-8">
          <div class="flex space-x-4">
            <button
              onClick={() => setActiveView('projects')}
              class={`px-4 py-2 rounded-md font-medium transition-colors ${
                activeView() === 'projects'
                  ? 'bg-blue-500 text-white'
                  : 'bg-white text-gray-700 hover:bg-gray-50 border border-gray-300'
              }`}
            >
              Projects
            </button>
            <button
              onClick={() => setActiveView('create')}
              class={`px-4 py-2 rounded-md font-medium transition-colors ${
                activeView() === 'create'
                  ? 'bg-blue-500 text-white'
                  : 'bg-white text-gray-700 hover:bg-gray-50 border border-gray-300'
              }`}
            >
              Create Project
            </button>
          </div>
        </nav>

        <main class="bg-white rounded-lg shadow-sm border border-gray-200 p-6">
          {activeView() === 'projects' ? (
            <ProjectList key={refreshKey()} onRefresh={handleRefresh} />
          ) : (
            <CreateProjectForm onProjectCreated={handleProjectCreated} />
          )}
        </main>

        <footer class="mt-8 text-center text-sm text-gray-500">
          <p>nocodo Manager - Minimal Web Interface</p>
        </footer>
      </div>
    </div>
  );
};

export default App;
