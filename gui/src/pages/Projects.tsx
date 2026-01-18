import { type Component } from 'solid-js';
import { A } from '@solidjs/router';

const Projects: Component = () => {
  const handleCreateProject = () => {
    // TODO: Implement project creation when API is ready
    console.log('Create project clicked');
  };

  return (
    <div class="container mx-auto p-8">
      <div class="flex flex-col items-center justify-center min-h-[60vh]">
        <h1 class="text-4xl font-bold mb-8">Projects</h1>
        <p class="text-lg text-base-content/70 mb-8 text-center max-w-md">
          Organize your tools, agents, and workflows into projects for better
          management and collaboration.
        </p>
        <button class="btn btn-primary btn-lg" onClick={handleCreateProject}>
          Create a Project
        </button>

        {/* Temporary test link - will be replaced with actual project list */}
        <div class="mt-8">
          <p class="text-sm text-base-content/50 mb-2">For testing:</p>
          <A href="/projects/123/workflow" class="link link-primary">
            View test project
          </A>
        </div>
      </div>
    </div>
  );
};

export default Projects;
