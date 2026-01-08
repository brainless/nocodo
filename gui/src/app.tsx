import { type Component, type ParentComponent } from 'solid-js';
import { Router, Route } from '@solidjs/router';
import Navigation from './components/Navigation';
import Agents from './pages/Agents';
import Home from './pages/Home';
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
      <Route path="/agents" component={Agents} />
      <Route path="/settings" component={Settings} />
      <Route path="/session/:id" component={SessionDetails} />
    </Router>
  );
};

export default App;
