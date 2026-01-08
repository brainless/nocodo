import { type Component } from "solid-js";

const Home: Component = () => {
  return (
    <div class="hero min-h-screen bg-base-200">
      <div class="hero-content text-center">
        <div class="max-w-md">
          <h1 class="text-5xl font-bold">Welcome to Nocodo</h1>
          <p class="py-6">
            Manage your agents, projects, and workflows with ease.
          </p>
          <button class="btn btn-primary">Get Started</button>
        </div>
      </div>
    </div>
  );
};

export default Home;
