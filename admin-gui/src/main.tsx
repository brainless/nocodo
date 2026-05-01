import { render } from 'solid-js/web';
import { Router, Route } from '@solidjs/router';
import './index.css';
import AdminHomePage from './pages/AdminHomePage';
import ProjectLayout from './components/ProjectLayout';
import DBDeveloperPage from './pages/DBDeveloperPage';
import ProjectManagerPage from './pages/ProjectManagerPage';
import BackendDeveloperPage from './pages/BackendDeveloperPage';
import UIDesignerPage from './pages/UIDesignerPage';
import ProjectSettingsPage from './pages/ProjectSettingsPage';

render(
  () => (
    <Router base="/admin">
      <Route path="/" component={AdminHomePage} />
      <Route path="/projects/:projectId" component={ProjectLayout}>
        <Route path="/manager" component={ProjectManagerPage} />
        <Route path="/db-developer" component={DBDeveloperPage} />
        <Route path="/backend" component={BackendDeveloperPage} />
        <Route path="/ui" component={UIDesignerPage} />
        <Route path="/settings" component={ProjectSettingsPage} />
      </Route>
    </Router>
  ),
  document.getElementById('root') as HTMLElement,
);
