import { createResource } from 'solid-js';
import type { HeartbeatResponse } from './types/api';

async function fetchHeartbeat(): Promise<HeartbeatResponse> {
  const res = await fetch('http://127.0.0.1:8080/api/heartbeat');
  if (!res.ok) {
    throw new Error(`heartbeat failed: ${res.status}`);
  }
  return res.json();
}

export default function App() {
  const [heartbeat] = createResource(fetchHeartbeat);

  return (
    <main class="min-h-screen bg-base-200 p-8">
      <section class="mx-auto mt-16 max-w-xl rounded-xl border border-base-300 bg-base-100 p-8 shadow-sm">
        <h1 class="text-3xl font-bold">Hello World</h1>
        <p class="mt-4 text-sm text-base-content/80">Backend heartbeat status:</p>
        <div class="mt-2">
          {heartbeat.loading && <span class="badge badge-warning">checking...</span>}
          {heartbeat.error && <span class="badge badge-error">offline</span>}
          {heartbeat() && (
            <span class="badge badge-success">
              {heartbeat()!.status} ({heartbeat()!.service})
            </span>
          )}
        </div>
      </section>
    </main>
  );
}
