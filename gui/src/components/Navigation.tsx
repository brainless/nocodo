import { A } from '@solidjs/router';

const Navigation = () => {
  return (
    <div class="navbar bg-base-100 shadow-sm">
      <div class="navbar-start">
        <ul class="menu menu-horizontal px-1">
          <li>
            <A href="/">Home</A>
          </li>
          <li>
            <A href="/sessions">Sessions</A>
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
  );
};

export default Navigation;
