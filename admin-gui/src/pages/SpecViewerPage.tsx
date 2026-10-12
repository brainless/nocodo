import { For, Show, createEffect, createMemo, createSignal, onCleanup } from 'solid-js';
import { useParams, useNavigate } from '@solidjs/router';
import { useProject } from '../contexts/ProjectContext';
import ProjectTopNav from '../components/ProjectTopNav';

// Read-only file tree + code viewer for generated `nocodo_praxis` artifacts
// (MILESTONE_PRAXIS_v1.md §9.2). Backed by GET /api/rust-engineer/praxis-specs,
// which lists `backend/src/praxis/{specs,transitions}/*.rs` and top-level
// `mod.rs` from the managed project. No write capability here — this page
// only reads and displays.

type PraxisSpecFile = {
  path: string; // e.g. "specs/personas.rs", "transitions/assign_task.rs", "mod.rs"
  content: string;
};

type PraxisSpecsResponse = {
  files: PraxisSpecFile[];
};

// Matches the `id: "123",` line inside a `Provenance::Conversation { .. }`
// block emitted by agents/src/spec_fact/*.rs — used to render a clickable
// link back to the chat session that produced the fact.
const PROVENANCE_ID_RE = /id:\s*"(\d+)"/;

type TreeGroup = {
  label: string;
  files: PraxisSpecFile[];
};

function groupFiles(files: PraxisSpecFile[]): { root: PraxisSpecFile[]; groups: TreeGroup[] } {
  const root: PraxisSpecFile[] = [];
  const byDir = new Map<string, PraxisSpecFile[]>();
  for (const f of files) {
    const slash = f.path.indexOf('/');
    if (slash === -1) {
      root.push(f);
      continue;
    }
    const dir = f.path.slice(0, slash);
    const list = byDir.get(dir) ?? [];
    list.push(f);
    byDir.set(dir, list);
  }
  // Preferred order: specs before transitions, then anything else alphabetically.
  const order = ['specs', 'transitions'];
  const dirs = [...byDir.keys()].sort((a, b) => {
    const ia = order.indexOf(a);
    const ib = order.indexOf(b);
    if (ia !== -1 || ib !== -1) return (ia === -1 ? 99 : ia) - (ib === -1 ? 99 : ib);
    return a.localeCompare(b);
  });
  const groups = dirs.map((label) => ({ label, files: byDir.get(label)! }));
  return { root, groups };
}

function CodeWithProvenance(props: { content: string; onNavigateSession: (sessionId: number) => void }) {
  const lines = createMemo(() => props.content.split('\n'));
  return (
    <pre class="text-xs font-mono bg-base-300 rounded p-3 overflow-x-auto whitespace-pre">
      <For each={lines()}>
        {(line) => {
          const m = line.match(PROVENANCE_ID_RE);
          if (!m) return <div>{line || ' '}</div>;
          const sessionId = parseInt(m[1], 10);
          const [before, after] = [
            line.slice(0, m.index),
            line.slice((m.index ?? 0) + m[0].length),
          ];
          return (
            <div>
              {before}
              <span>id: "</span>
              <button
                type="button"
                class="link link-primary font-mono"
                title="Jump to the chat session that produced this fact"
                onClick={() => props.onNavigateSession(sessionId)}
              >
                {m[1]}
              </button>
              <span>"</span>
              {after}
            </div>
          );
        }}
      </For>
    </pre>
  );
}

export default function SpecViewerPage() {
  const { currentProject } = useProject();
  const params = useParams<{ projectId: string }>();
  const navigate = useNavigate();

  const [files, setFiles] = createSignal<PraxisSpecFile[]>([]);
  const [selectedPath, setSelectedPath] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  let abortController: AbortController | null = null;

  const fetchFiles = async (projectId: number) => {
    abortController?.abort();
    const ac = new AbortController();
    abortController = ac;
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/rust-engineer/praxis-specs?project_id=${projectId}`, {
        signal: ac.signal,
      });
      if (!res.ok) {
        setError(`Failed to load praxis specs (${res.status})`);
        return;
      }
      const data = (await res.json()) as PraxisSpecsResponse;
      const list = data.files ?? [];
      setFiles(list);
      setSelectedPath((prev) => {
        if (prev && list.some((f) => f.path === prev)) return prev;
        return list[0]?.path ?? null;
      });
    } catch (err) {
      if ((err as Error).name !== 'AbortError') {
        setError('Failed to load praxis specs');
      }
    } finally {
      setLoading(false);
    }
  };

  createEffect(() => {
    const pid = currentProject()?.id;
    if (pid) void fetchFiles(pid);
  });

  onCleanup(() => abortController?.abort());

  const tree = createMemo(() => groupFiles(files()));
  const selectedFile = createMemo(() => files().find((f) => f.path === selectedPath()) ?? null);

  const goToSession = (sessionId: number) => {
    navigate(`/projects/${params.projectId}/chat/${sessionId}`);
  };

  return (
    <main class="sheet-app">
      <section class="sheet-main">
        <ProjectTopNav
          title="Spec Viewer"
          actions={
            <button
              class="btn btn-sm"
              disabled={loading() || !currentProject()?.id}
              onClick={() => {
                const pid = currentProject()?.id;
                if (pid) void fetchFiles(pid);
              }}
            >
              <Show when={loading()}>
                <span class="loading loading-spinner loading-xs" />
              </Show>
              Refresh
            </button>
          }
        />

        <Show when={error()}>
          <div class="p-4">
            <div class="alert alert-error alert-sm">
              <span>{error()}</span>
            </div>
          </div>
        </Show>

        <div class="flex flex-1 min-h-0 overflow-hidden">
          {/* File tree */}
          <div class="w-64 shrink-0 border-r border-base-200 overflow-y-auto p-2">
            <Show
              when={files().length > 0}
              fallback={
                <p class="text-xs text-base-content/40 italic p-2">
                  <Show when={!loading()} fallback="Loading…">
                    No praxis spec files generated yet.
                  </Show>
                </p>
              }
            >
              <For each={tree().root}>
                {(f) => (
                  <button
                    type="button"
                    class={`block w-full text-left px-2 py-1 rounded text-xs font-mono truncate ${
                      selectedPath() === f.path ? 'bg-primary/10 text-primary' : 'hover:bg-base-200'
                    }`}
                    onClick={() => setSelectedPath(f.path)}
                  >
                    {f.path}
                  </button>
                )}
              </For>
              <For each={tree().groups}>
                {(group) => (
                  <div class="mt-2">
                    <div class="px-2 py-1 text-[10px] uppercase tracking-wide text-base-content/40">
                      praxis/{group.label}/
                    </div>
                    <For each={group.files}>
                      {(f) => {
                        const name = f.path.slice(f.path.indexOf('/') + 1);
                        return (
                          <button
                            type="button"
                            class={`block w-full text-left px-3 py-1 rounded text-xs font-mono truncate ${
                              selectedPath() === f.path
                                ? 'bg-primary/10 text-primary'
                                : 'hover:bg-base-200'
                            }`}
                            onClick={() => setSelectedPath(f.path)}
                          >
                            {name}
                          </button>
                        );
                      }}
                    </For>
                  </div>
                )}
              </For>
            </Show>
          </div>

          {/* Code viewer */}
          <div class="flex-1 min-w-0 overflow-y-auto p-4">
            <Show
              when={selectedFile()}
              fallback={
                <div class="flex items-center justify-center h-64 text-base-content/30 text-sm">
                  Select a file to view its generated code.
                </div>
              }
            >
              {(f) => (
                <div class="flex flex-col gap-2">
                  <h3 class="text-sm font-semibold text-base-content/60 font-mono">
                    praxis/{f().path}
                  </h3>
                  <CodeWithProvenance content={f().content} onNavigateSession={goToSession} />
                </div>
              )}
            </Show>
          </div>
        </div>
      </section>
    </main>
  );
}
