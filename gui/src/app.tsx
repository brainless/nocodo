import { type Component, type ParentComponent } from 'solid-js';
import { Router, Route, A } from '@solidjs/router';
import Agents from './pages/Agents';
import Home from './pages/Home';
import Settings from './pages/Settings';

const Layout: ParentComponent = (props) => {
  return (
    <>
      <div class="navbar bg-base-100 shadow-sm">
        <div class="navbar-start">
          <ul class="menu menu-horizontal px-1">
            <li>
              <A href="/">Home</A>
            </li>
            <li>
              <a>Projects</a>
            </li>
          </ul>
          <input
            type="text"
            placeholder="Search"
            class="input input-bordered w-24 md:w-auto ml-4"
          />
        </div>
        <div class="navbar-end">
          <ul class="menu menu-horizontal px-1">
            <li>
              <A href="/agents">Agents</A>
            </li>
            <li>
              <A href="/settings">Settings</A>
            </li>
          </ul>
        </div>
      </div>
      <div class="min-h-screen">{props.children}</div>
    </>
  );
};

const App: Component = () => {
  return (
    <Router root={Layout}>
      <Route path="/" component={Home} />
      <Route path="/agents" component={Agents} />
      <Route path="/settings" component={Settings} />
    </Router>
  );
};

export default App;
