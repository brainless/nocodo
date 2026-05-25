import { render } from 'solid-js/web';
import { Router, Route } from '@solidjs/router';
import { Show } from 'solid-js';
import './index.css';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import { AuthModal } from './components/AuthModal';
import AdminHomePage from './pages/AdminHomePage';
import ComponentsGalleryPage from './pages/ComponentsGalleryPage';
import ProjectLayout from './components/ProjectLayout';
import DatabasePage from './pages/DatabasePage';
import EpicsPage from './pages/EpicsPage';
import BackendPage from './pages/BackendPage';
import UIDesignPage from './pages/UIDesignPage';
import ProjectSettingsPage from './pages/ProjectSettingsPage';
import UserChatPage from './pages/UserChatPage';
import StackNotesPage from './pages/StackNotesPage';
import ProjectNotesPage from './pages/ProjectNotesPage';
import RustEngineerPage from './pages/RustEngineerPage';

function AppRoutes() {
  return (
    <Router base="/admin">
      <Route path="/" component={AdminHomePage} />
      <Route path="/components" component={ComponentsGalleryPage} />
      <Route path="/projects/:projectId" component={ProjectLayout}>
        <Route path="/epics" component={EpicsPage} />
        <Route path="/epics/epic/:epicId" component={EpicsPage} />
        <Route path="/epics/task/:taskId" component={EpicsPage} />
        <Route path="/database" component={DatabasePage} />
        <Route path="/backend" component={BackendPage} />
        <Route path="/ui-design" component={UIDesignPage} />
        <Route path="/chat" component={UserChatPage} />
        <Route path="/chat/sessions" component={UserChatPage} />
        <Route path="/chat/:sessionId" component={UserChatPage} />
        <Route path="/rust-engineer" component={RustEngineerPage} />
        <Route path="/stack-notes" component={StackNotesPage} />
        <Route path="/project-notes" component={ProjectNotesPage} />
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
