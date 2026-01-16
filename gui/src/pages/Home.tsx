import { createSignal, type Component } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import type {
  AgentExecutionRequest,
  AgentExecutionResponse,
} from '../../api-types/types';

const Home: Component = () => {
  const [userPrompt, setUserPrompt] = createSignal('');
  const [isSubmitting, setIsSubmitting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const navigate = useNavigate();

  const handleSubmit = async (e: Event) => {
    e.preventDefault();

    const prompt = userPrompt().trim();
    if (!prompt) {
      setError('Please describe your workflow');
      return;
    }

    setIsSubmitting(true);
    setError(null);

    try {
      const requestBody: AgentExecutionRequest = {
        user_prompt: prompt,
        config: {
          type: 'structured-json',
          type_names: ['Workflow', 'WorkflowStep', 'WorkflowWithSteps'],
          domain_description: 'Workflow automation and task management',
        },
      };

      const response = await fetch(
        'http://127.0.0.1:8080/agents/workflow-creation/execute',
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(requestBody),
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to start agent: ${response.statusText}`);
      }

      const result: AgentExecutionResponse = await response.json();

      // Navigate to workflow page with session_id
      // Use a dummy project_id=999 for demo purposes
      navigate(`/projects/999/workflow?session_id=${result.session_id}`);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'Failed to generate workflow'
      );
      setIsSubmitting(false);
    }
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
              value={userPrompt()}
              onInput={(e) => setUserPrompt(e.currentTarget.value)}
              disabled={isSubmitting()}
            ></textarea>
          </div>

          {error() && (
            <div class="alert alert-error">
              <span>{error()}</span>
            </div>
          )}

          <div class="form-control">
            <button
              type="submit"
              class="btn btn-primary btn-lg w-full"
              disabled={isSubmitting()}
            >
              {isSubmitting() ? (
                <>
                  <span class="loading loading-spinner"></span>
                  Generating Workflow...
                </>
              ) : (
                'Generate Agent'
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default Home;
