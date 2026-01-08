import { createSignal, onMount, type Component } from "solid-js";
import type { AgentInfo, AgentsResponse } from "../../api-types/types";

const Agents: Component = () => {
  const [agents, setAgents] = createSignal<AgentInfo[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      const response = await fetch("http://127.0.0.1:8080/agents");
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data: AgentsResponse = await response.json();
      setAgents(data.agents);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to fetch agents");
    } finally {
      setLoading(false);
    }
  });

  return (
    <div class="container mx-auto p-8">
      <h1 class="text-3xl font-bold mb-6">Agents</h1>
      
      {loading() && (
        <div class="flex justify-center">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
      )}

      {error() && (
        <div class="alert alert-error">
          <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <span>{error()}</span>
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
