import { type Component } from "solid-js";

const App: Component = () => {
  return (
    <>
      <div class="navbar bg-base-100 shadow-sm">
        <div class="navbar-start">
          <ul class="menu menu-horizontal px-1">
            <li><a>Home</a></li>
            <li><a>Projects</a></li>
          </ul>
          <input type="text" placeholder="Search" class="input input-bordered w-24 md:w-auto ml-4" />
        </div>
        <div class="navbar-end">
          <ul class="menu menu-horizontal px-1">
            <li><a>Agents</a></li>
            <li><a>Settings</a></li>
          </ul>
        </div>
      </div>
      <div class="min-h-screen bg-white">
        {/* Blank page */}
      </div>
    </>
  );
};

export default App;
