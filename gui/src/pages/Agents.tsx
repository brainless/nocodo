import { createSignal, onMount, type Component, Show } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import type {
  AgentInfo,
  AgentsResponse,
  AgentConfig,
  AgentExecutionRequest,
  AgentExecutionResponse,
} from '../../api-types/types';

const Agents: Component = () => {
  const navigate = useNavigate();
  const [agents, setAgents] = createSignal<AgentInfo[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [executing, setExecuting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const [userPrompt, setUserPrompt] = createSignal('');
  const [selectedAgentId, setSelectedAgentId] = createSignal<string>('');
  const [dbPath, setDbPath] = createSignal('');
  const [codebasePath, setCodebasePath] = createSignal('');
  const [imagePath, setImagePath] = createSignal('');

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
      } else if (agentId === 'tesseract') {
        if (!imagePath().trim()) {
          throw new Error('Please enter an image path');
        }
        config = {
          type: 'tesseract',
          image_path: imagePath(),
        };
        endpoint = 'http://127.0.0.1:8080/agents/tesseract/execute';
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
    <div class="container mx-auto p-8">
      <h1 class="text-3xl font-bold mb-6">Agents</h1>

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
        <form onSubmit={handleSubmit} class="space-y-6 mb-12">
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

          <Show when={selectedAgentId() === 'tesseract'}>
            <div class="form-control">
              <label class="label">
                <span class="label-text">Image Path</span>
              </label>
              <input
                type="text"
                placeholder="Enter image path (PNG, JPG, PDF, TIFF)"
                class="input input-bordered w-full"
                value={imagePath()}
                onInput={(e) => setImagePath(e.currentTarget.value)}
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

      {loading() && (
        <div class="flex justify-center">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
      )}

      {!loading() && !error() && (
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {agents().map((agent) => (
            <div class="card bg-base-100 shadow-xl">
              <div class="card-body">
                <h2 class="card-title">
                  {agent.name}
                  {agent.enabled ? (
                    <div class="badge badge-success">Enabled</div>
                  ) : (
                    <div class="badge badge-error">Disabled</div>
                  )}
                </h2>
                <p>{agent.description}</p>
                <div class="card-actions justify-end">
                  <button class="btn btn-primary" disabled={!agent.enabled}>
                    Run Agent
                  </button>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {!loading() && !error() && agents().length === 0 && (
        <div class="text-center text-base-content/70">
          <p>No agents available</p>
        </div>
      )}
    </div>
  );
};

export default Agents;
