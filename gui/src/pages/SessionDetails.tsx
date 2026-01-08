import { createSignal, onMount, type Component, For, Show } from 'solid-js';
import { useParams } from '@solidjs/router';
import type { SessionResponse } from '../../api-types/types';

const SessionDetails: Component = () => {
  const params = useParams();
  const [session, setSession] = createSignal<SessionResponse | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    try {
      const sessionId = params.id;
      const response = await fetch(
        `http://127.0.0.1:8080/agents/sessions/${sessionId}`
      );
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data: SessionResponse = await response.json();
      setSession(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to fetch session');
    } finally {
      setLoading(false);
    }
  });

  const formatTimestamp = (timestamp: bigint | number) => {
    const date = new Date(Number(timestamp) * 1000);
    return date.toLocaleString();
  };

  const getStatusBadgeClass = (status: string) => {
    switch (status.toLowerCase()) {
      case 'completed':
        return 'badge badge-success';
      case 'running':
        return 'badge badge-info';
      case 'failed':
        return 'badge badge-error';
      default:
        return 'badge badge-ghost';
    }
  };

  return (
    <div class="container mx-auto p-8 max-w-6xl">
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

      {!loading() && session() && (
        <div class="space-y-6">
          <div class="card bg-base-100 shadow-xl">
            <div class="card-body">
              <h2 class="card-title text-2xl">Session Details</h2>
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
                <div>
                  <p class="text-sm text-base-content/70">Agent</p>
                  <p class="font-semibold">{session()!.agent_name}</p>
                </div>
                <div>
                  <p class="text-sm text-base-content/70">Status</p>
                  <div class={getStatusBadgeClass(session()!.status)}>
                    {session()!.status}
                  </div>
                </div>
                <div>
                  <p class="text-sm text-base-content/70">Provider</p>
                  <p class="font-semibold">{session()!.provider}</p>
                </div>
                <div>
                  <p class="text-sm text-base-content/70">Model</p>
                  <p class="font-semibold">{session()!.model}</p>
                </div>
              </div>
              <div class="mt-4">
                <p class="text-sm text-base-content/70">User Prompt</p>
                <p class="mt-2 p-4 bg-base-200 rounded-lg">
                  {session()!.user_prompt}
                </p>
              </div>
              <Show when={session()!.result}>
                <div class="mt-4">
                  <p class="text-sm text-base-content/70">Result</p>
                  <p class="mt-2 p-4 bg-base-200 rounded-lg whitespace-pre-wrap">
                    {session()!.result}
                  </p>
                </div>
              </Show>
            </div>
          </div>

          <Show when={session()!.messages.length > 0}>
            <div class="card bg-base-100 shadow-xl">
              <div class="card-body">
                <h3 class="card-title">Messages</h3>
                <div class="space-y-4">
                  <For each={session()!.messages}>
                    {(message) => (
                      <div class="p-4 bg-base-200 rounded-lg">
                        <div class="flex justify-between items-center mb-2">
                          <span class="badge badge-sm">{message.role}</span>
                          <span class="text-xs text-base-content/70">
                            {formatTimestamp(message.created_at)}
                          </span>
                        </div>
                        <p class="whitespace-pre-wrap">{message.content}</p>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            </div>
          </Show>

          <Show when={session()!.tool_calls.length > 0}>
            <div class="card bg-base-100 shadow-xl">
              <div class="card-body">
                <h3 class="card-title">Tool Calls</h3>
                <div class="space-y-4">
                  <For each={session()!.tool_calls}>
                    {(toolCall) => (
                      <div class="p-4 bg-base-200 rounded-lg">
                        <div class="flex justify-between items-center mb-2">
                          <span class="font-semibold">
                            {toolCall.tool_name}
                          </span>
                          <div class="flex gap-2 items-center">
                            <span class={getStatusBadgeClass(toolCall.status)}>
                              {toolCall.status}
                            </span>
                            {toolCall.execution_time_ms && (
                              <span class="text-xs text-base-content/70">
                                {Number(toolCall.execution_time_ms)}ms
                              </span>
                            )}
                          </div>
                        </div>
                        <div class="mt-2">
                          <p class="text-xs text-base-content/70 mb-1">
                            Request:
                          </p>
                          <pre class="text-xs bg-base-300 p-2 rounded overflow-x-auto">
                            {JSON.stringify(toolCall.request, null, 2)}
                          </pre>
                        </div>
                        {toolCall.response && (
                          <div class="mt-2">
                            <p class="text-xs text-base-content/70 mb-1">
                              Response:
                            </p>
                            <pre class="text-xs bg-base-300 p-2 rounded overflow-x-auto">
                              {JSON.stringify(toolCall.response, null, 2)}
                            </pre>
                          </div>
                        )}
                      </div>
                    )}
                  </For>
                </div>
              </div>
            </div>
          </Show>
        </div>
      )}

      {!loading() && !session() && (
        <div class="text-center text-base-content/70">
          <p>Session not found</p>
        </div>
      )}
    </div>
  );
};

export default SessionDetails;
