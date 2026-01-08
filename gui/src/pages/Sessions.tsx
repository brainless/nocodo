import { createSignal, onMount, type Component } from 'solid-js';
import { A } from '@solidjs/router';
import type {
  SessionListItem,
  SessionListResponse,
} from '../../api-types/types';

const Sessions: Component = () => {
  const [sessions, setSessions] = createSignal<SessionListItem[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const formatTimestamp = (timestamp: bigint | number) => {
    const date = new Date(Number(timestamp) * 1000);
    const now = new Date();
    const diffInSeconds = Math.floor((now.getTime() - date.getTime()) / 1000);

    if (diffInSeconds < 60) {
      return 'just now';
    } else if (diffInSeconds < 3600) {
      const minutes = Math.floor(diffInSeconds / 60);
      return `${minutes} minute${minutes > 1 ? 's' : ''} ago`;
    } else if (diffInSeconds < 86400) {
      const hours = Math.floor(diffInSeconds / 3600);
      return `${hours} hour${hours > 1 ? 's' : ''} ago`;
    } else if (diffInSeconds < 604800) {
      const days = Math.floor(diffInSeconds / 86400);
      return `${days} day${days > 1 ? 's' : ''} ago`;
    } else {
      return date.toLocaleDateString();
    }
  };

  const truncatePrompt = (prompt: string, maxLength: number = 100) => {
    if (prompt.length <= maxLength) {
      return prompt;
    }
    return prompt.substring(0, maxLength) + '...';
  };

  onMount(async () => {
    try {
      const response = await fetch('http://127.0.0.1:8080/agents/sessions');
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data: SessionListResponse = await response.json();
      setSessions(data.sessions);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to fetch sessions');
    } finally {
      setLoading(false);
    }
  });

  return (
    <div class="container mx-auto p-8">
      <h1 class="text-3xl font-bold mb-6">Sessions</h1>

      {loading() && (
        <div class="flex justify-center">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
      )}

      {error() && (
        <div class="alert alert-error">
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

      {!loading() && !error() && (
        <div class="grid grid-cols-1 gap-6">
          {sessions().map((session) => (
            <div class="card bg-base-100 shadow-xl hover:shadow-2xl transition-shadow duration-200">
              <div class="card-body">
                <div class="flex items-start justify-between mb-2">
                  <h2 class="card-title text-lg">{session.agent_name}</h2>
                  <div class="badge badge-outline badge-sm">
                    ID: {session.id}
                  </div>
                </div>

                <div class="mb-4">
                  <p class="text-sm text-base-content/70 mb-2">
                    Initial prompt:
                  </p>
                  <p class="text-base-content/90 line-clamp-3">
                    {truncatePrompt(session.user_prompt)}
                  </p>
                </div>

                <div class="flex items-center justify-between text-sm text-base-content/60">
                  <div class="flex items-center gap-1">
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      class="h-4 w-4"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                    <span>{formatTimestamp(session.started_at)}</span>
                  </div>
                </div>

                <div class="card-actions justify-end mt-4">
                  <A
                    href={`/session/${session.id}`}
                    class="btn btn-primary btn-sm"
                  >
                    View Details
                  </A>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {!loading() && !error() && sessions().length === 0 && (
        <div class="text-center text-base-content/70 py-12">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-16 w-16 mx-auto mb-4 text-base-content/40"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <p class="text-lg">No sessions found</p>
          <p class="text-sm mt-2">
            Start a conversation with an agent to see your sessions here.
          </p>
        </div>
      )}
    </div>
  );
};

export default Sessions;
