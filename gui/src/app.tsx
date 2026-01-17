import { type Component, type ParentComponent } from 'solid-js';
import { Router, Route } from '@solidjs/router';
import Navigation from './components/Navigation';
import ProjectDetailsLayout from './components/ProjectDetailsLayout';
import Agents from './pages/Agents';
import Home from './pages/Home';
import Projects from './pages/Projects';
import ProjectWorkflow from './pages/ProjectWorkflow';
import ProjectProcess from './pages/ProjectProcess';
import ProjectDataSources from './pages/ProjectDataSources';
import ProjectSpecifications from './pages/ProjectSpecifications';
import Settings from './pages/Settings';
import SessionDetails from './pages/SessionDetails';
import Sessions from './pages/Sessions';

const Layout: ParentComponent = (props) => {
  return (
    <>
      <Navigation />
      <div class="min-h-screen">{props.children}</div>
    </>
  );
};

const App: Component = () => {
  return (
    <Router root={Layout}>
      <Route path="/" component={Home} />
      <Route path="/sessions" component={Sessions} />
      <Route path="/projects" component={Projects} />
      <Route
        path="/projects/:projectId"
        component={ProjectDetailsLayout}
        matchFilters={{ projectId: (id: string) => /^\d+$/.test(id) }}
      >
        <Route path="/specifications" component={ProjectSpecifications} />
        <Route path="/workflow" component={ProjectWorkflow} />
        <Route path="/process" component={ProjectProcess} />
        <Route path="/data-sources" component={ProjectDataSources} />
      </Route>
      <Route path="/agents" component={Agents} />
      <Route path="/settings" component={Settings} />
      <Route path="/session/:id" component={SessionDetails} />
    </Router>
  );
};

export default App;
