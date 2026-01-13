import type { Component } from 'solid-js';

const Home: Component = () => {
  const handleSubmit = (e: Event) => {
    e.preventDefault();
  };

  return (
    <div class="min-h-screen bg-base-200 flex items-center justify-center p-8">
      <div class="w-full max-w-3xl">
        <h1 class="text-4xl font-bold mb-4">Let's automate your workflow</h1>
        <p class="text-lg text-base-content/70 mb-8">
          Describe your workflow, mention what or how you scan emails, messages,
          files, etc. and use databases, APIs or other systems to get work done.
          What is the objective of your workflow. It is OK if the workflow is
          part of a larger set of processes.
        </p>

        <form onSubmit={handleSubmit} class="space-y-6">
          <div class="form-control">
            <textarea
              class="textarea textarea-bordered w-full h-48 text-lg resize-none"
              placeholder="Describe your workflow here..."
            ></textarea>
          </div>

          <div class="form-control">
            <button type="submit" class="btn btn-primary btn-lg w-full">
              Generate Agent
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default Home;
