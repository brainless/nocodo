import { Show, createSignal } from 'solid-js';
import { useProject } from '../contexts/ProjectContext';
import ProjectTopNav from '../components/ProjectTopNav';

type RunResult = {
  prompt: string;
  raw_response: string;
  code: string | null;
};

export default function RustEngineerPage() {
  const { currentProject } = useProject();
  const [structName, setStructName] = createSignal('');
  const [fnName, setFnName] = createSignal('');
  const [running, setRunning] = createSignal(false);
  const [result, setResult] = createSignal<RunResult | null>(null);
  const [error, setError] = createSignal<string | null>(null);

  const canRun = () => structName().trim().length > 0 && fnName().trim().length > 0;

  const run = async () => {
    const pid = currentProject()?.id;
    if (!pid || !canRun()) return;
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const res = await fetch('/api/rust-engineer/run', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          project_id: pid,
          struct_name: structName().trim(),
          fn_name: fnName().trim(),
        }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error ?? `Error ${res.status}`);
        return;
      }
      setResult(data as RunResult);
    } catch (err) {
      setError(String(err));
    } finally {
      setRunning(false);
    }
  };

  const actions = (
    <div class="flex items-center gap-2">
      <input
        type="text"
        placeholder="StructName"
        class="input input-sm input-bordered w-36 font-mono"
        value={structName()}
        onInput={(e) => setStructName(e.currentTarget.value)}
        onKeyDown={(e) => e.key === 'Enter' && canRun() && run()}
      />
      <input
        type="text"
        placeholder="fn_name"
        class="input input-sm input-bordered w-36 font-mono"
        value={fnName()}
        onInput={(e) => setFnName(e.currentTarget.value)}
        onKeyDown={(e) => e.key === 'Enter' && canRun() && run()}
      />
      <button
        class="btn btn-sm btn-primary"
        disabled={!canRun() || running()}
        onClick={run}
      >
        <Show when={running()}>
          <span class="loading loading-spinner loading-xs" />
        </Show>
        Run
      </button>
    </div>
  );

  return (
    <main class="sheet-app">
      <section class="sheet-main">
        <ProjectTopNav title="Rust Engineer" actions={actions} />

        <Show when={error()}>
          <div class="p-4">
            <div class="alert alert-error alert-sm">
              <span>{error()}</span>
            </div>
          </div>
        </Show>

        <Show when={result()}>
          {(r) => (
            <div class="p-4 flex flex-col gap-4">
              <div class="card bg-base-200 rounded-lg">
                <div class="card-body p-4">
                  <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide mb-2">
                    Prompt sent
                  </h3>
                  <pre class="text-xs font-mono bg-base-300 rounded p-3 overflow-x-auto whitespace-pre-wrap break-words max-h-80 overflow-y-auto">
                    {r().prompt}
                  </pre>
                </div>
              </div>

              <div class="card bg-base-200 rounded-lg">
                <div class="card-body p-4">
                  <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide mb-2">
                    Raw response
                  </h3>
                  <pre class="text-xs font-mono bg-base-300 rounded p-3 overflow-x-auto whitespace-pre-wrap break-words max-h-80 overflow-y-auto">
                    {r().raw_response}
                  </pre>
                </div>
              </div>

              <Show when={r().code}>
                {(code) => (
                  <div class="card bg-base-200 rounded-lg">
                    <div class="card-body p-4">
                      <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide mb-2">
                        Extracted code
                      </h3>
                      <pre class="text-xs font-mono bg-base-300 rounded p-3 overflow-x-auto whitespace-pre-wrap break-words">
                        {code()}
                      </pre>
                    </div>
                  </div>
                )}
              </Show>
            </div>
          )}
        </Show>

        <Show when={!result() && !running() && !error()}>
          <div class="flex items-center justify-center h-64 text-base-content/30 text-sm">
            Enter a struct name and function name above, then click Run.
          </div>
        </Show>
      </section>
    </main>
  );
}
