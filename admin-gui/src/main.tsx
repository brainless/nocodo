import { render } from 'solid-js/web';
import { Router, Route } from '@solidjs/router';
import { Show } from 'solid-js';
import './index.css';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import { AuthModal } from './components/AuthModal';
import AdminHomePage from './pages/AdminHomePage';
import ComponentsGalleryPage from './pages/ComponentsGalleryPage';
import ProjectLayout from './components/ProjectLayout';
import DBDeveloperPage from './pages/DBDeveloperPage';
import ProjectManagerPage from './pages/ProjectManagerPage';
import BackendDeveloperPage from './pages/BackendDeveloperPage';
import UIDesignerPage from './pages/UIDesignerPage';
import ProjectSettingsPage from './pages/ProjectSettingsPage';

function AppRoutes() {
  return (
    <Router base="/admin">
      <Route path="/" component={AdminHomePage} />
      <Route path="/components" component={ComponentsGalleryPage} />
      <Route path="/projects/:projectId" component={ProjectLayout}>
        <Route path="/manager" component={ProjectManagerPage} />
        <Route path="/db-developer" component={DBDeveloperPage} />
        <Route path="/backend" component={BackendDeveloperPage} />
        <Route path="/ui" component={UIDesignerPage} />
        <Route path="/settings" component={ProjectSettingsPage} />
      </Route>
    </Router>
  );
}

function App() {
  const { auth } = useAuth();
  return (
    <Show
      when={!auth().checking}
      fallback={
        <div class="flex items-center justify-center min-h-screen">
          <span class="loading loading-spinner loading-lg" />
        </div>
      }
    >
      <AuthModal />
      <Show when={!auth().authRequired || auth().isAuthenticated}>
        <AppRoutes />
      </Show>
    </Show>
  );
}

render(
  () => (
    <AuthProvider>
      <App />
    </AuthProvider>
  ),
  document.getElementById('root') as HTMLElement,
);
