import { render } from 'solid-js/web';
import { Router, Route } from '@solidjs/router';
import './index.css';
import AdminHomePage from './pages/AdminHomePage';
import { ProjectPage } from './App';

render(
  () => (
    <Router base="/admin">
      <Route path="/" component={AdminHomePage} />
      <Route path="/projects/:projectId" component={ProjectPage} />
    </Router>
  ),
  document.getElementById('root') as HTMLElement,
);
