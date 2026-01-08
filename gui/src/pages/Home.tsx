import { createSignal, onMount, type Component, Show } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import type {
  AgentsResponse,
  AgentInfo,
  AgentExecutionRequest,
  AgentExecutionResponse,
  AgentConfig,
} from '../../api-types/types';

const Home: Component = () => {
  const navigate = useNavigate();
  const [agents, setAgents] = createSignal<AgentInfo[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [executing, setExecuting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const [userPrompt, setUserPrompt] = createSignal('');
  const [selectedAgentId, setSelectedAgentId] = createSignal<string>('');
  const [dbPath, setDbPath] = createSignal('');
  const [codebasePath, setCodebasePath] = createSignal('');

  onMount(async () => {
    try {
      const response = await fetch('http://127.0.0.1:8080/agents');
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data: AgentsResponse = await response.json();
      setAgents(data.agents);
      if (data.agents.length > 0) {
        setSelectedAgentId(data.agents[0].id);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to fetch agents');
    } finally {
      setLoading(false);
    }
  });

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setExecuting(true);
    setError(null);

    try {
      const agentId = selectedAgentId();
      if (!agentId) {
        throw new Error('Please select an agent');
      }

      if (!userPrompt().trim()) {
        throw new Error('Please enter a prompt');
      }

      let config: AgentConfig;
      let endpoint: string;

      if (agentId === 'sqlite') {
        if (!dbPath().trim()) {
          throw new Error('Please enter a database path for SQLite agent');
        }
        config = {
          type: 'sqlite',
          db_path: dbPath(),
        };
        endpoint = 'http://127.0.0.1:8080/agents/sqlite/execute';
      } else if (agentId === 'codebase-analysis') {
        if (!codebasePath().trim()) {
          throw new Error('Please enter a codebase path');
        }
        config = {
          type: 'codebase-analysis',
          path: codebasePath(),
          max_depth: null,
        };
        endpoint = 'http://127.0.0.1:8080/agents/codebase-analysis/execute';
      } else {
        throw new Error('Unknown agent type');
      }

      const requestData: AgentExecutionRequest = {
        user_prompt: userPrompt(),
        config,
      };

      const response = await fetch(endpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(requestData),
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(
          errorData.error || `HTTP error! status: ${response.status}`
        );
      }

      const data: AgentExecutionResponse = await response.json();
      navigate(`/session/${data.session_id}`);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to execute agent');
    } finally {
      setExecuting(false);
    }
  };

  return (
    <div class="min-h-screen bg-base-200 flex items-center justify-center p-8">
      <div class="w-full max-w-3xl">
        {loading() && (
          <div class="flex justify-center">
            <span class="loading loading-spinner loading-lg"></span>
          </div>
        )}

        {error() && (
          <div class="alert alert-error mb-6">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              class="stroke-current shrink-0 h-6 w-6"
              fill="none"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <span>{error()}</span>
          </div>
        )}

        {!loading() && (
          <form onSubmit={handleSubmit} class="space-y-6">
            <div class="form-control">
              <textarea
                class="textarea textarea-bordered w-full h-48 text-lg resize-none"
                placeholder="Enter your prompt here..."
                value={userPrompt()}
                onInput={(e) => setUserPrompt(e.currentTarget.value)}
                disabled={executing()}
              ></textarea>
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">Select Agent</span>
              </label>
              <select
                class="select select-bordered w-full"
                value={selectedAgentId()}
                onChange={(e) => setSelectedAgentId(e.currentTarget.value)}
                disabled={executing()}
              >
                {agents().map((agent) => (
                  <option value={agent.id} disabled={!agent.enabled}>
                    {agent.name} - {agent.description}
                  </option>
                ))}
              </select>
            </div>

            <Show when={selectedAgentId() === 'sqlite'}>
              <div class="form-control">
                <label class="label">
                  <span class="label-text">Database Path</span>
                </label>
                <input
                  type="text"
                  placeholder="Enter SQLite database path"
                  class="input input-bordered w-full"
                  value={dbPath()}
                  onInput={(e) => setDbPath(e.currentTarget.value)}
                  disabled={executing()}
                />
              </div>
            </Show>

            <Show when={selectedAgentId() === 'codebase-analysis'}>
              <div class="form-control">
                <label class="label">
                  <span class="label-text">Codebase Path</span>
                </label>
                <input
                  type="text"
                  placeholder="Enter codebase path"
                  class="input input-bordered w-full"
                  value={codebasePath()}
                  onInput={(e) => setCodebasePath(e.currentTarget.value)}
                  disabled={executing()}
                />
              </div>
            </Show>

            <div class="form-control">
              <button
                type="submit"
                class="btn btn-primary btn-lg w-full"
                disabled={executing()}
              >
                {executing() ? (
                  <>
                    <span class="loading loading-spinner loading-sm"></span>
                    Running Agent...
                  </>
                ) : (
                  'Run Agent'
                )}
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
};

export default Home;
