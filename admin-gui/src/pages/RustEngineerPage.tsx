import { For, Show, createSignal } from 'solid-js';
import { useProject } from '../contexts/ProjectContext';
import ProjectTopNav from '../components/ProjectTopNav';

type Mode = 'diesel_model_struct' | 'state_transition';

type TransitionMetadata = {
  struct_name: string | null;
  has_unresolved: boolean;
  [key: string]: unknown;
};

type RunResult = {
  system_prompt?: string | null;
  prompt: string;
  raw_response: string;
  code: string | null;
  file_path?: string;
  metadata?: TransitionMetadata;
};

const MODES: { value: Mode; label: string; placeholder: string }[] = [
  {
    value: 'diesel_model_struct',
    label: 'Diesel Model Struct',
    placeholder: 'Describe the data model you want to generate a Diesel struct for...',
  },
  {
    value: 'state_transition',
    label: 'State Transition',
    placeholder:
      'Describe the action/transition to generate a nocodo_praxis TransitionGuard for (e.g. "assign a task to a team member")...',
  },
];

export default function RustEngineerPage() {
  const { currentProject } = useProject();
  const [mode, setMode] = createSignal<Mode>('diesel_model_struct');
  const [prompt, setPrompt] = createSignal('');
  const [running, setRunning] = createSignal(false);
  const [result, setResult] = createSignal<RunResult | null>(null);
  const [error, setError] = createSignal<string | null>(null);

  const canRun = () => prompt().trim().length > 0;

  const activeMode = () => MODES.find((m) => m.value === mode())!;

  const run = async () => {
    const pid = currentProject()?.id;
    if (!pid || !canRun()) return;
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const currentMode = mode();
      const res =
        currentMode === 'state_transition'
          ? await fetch('/api/rust-engineer/state-transition', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                project_id: pid,
                action_description: prompt().trim(),
                apply: true,
              }),
            })
          : await fetch('/api/rust-engineer/run', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                project_id: pid,
                mode: 'diesel_model_struct',
                prompt: `Write a Diesel SQLite read model struct for the following table:\n\n${prompt().trim()}`,
                apply: true,
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

  return (
    <main class="sheet-app">
      <section class="sheet-main">
        <ProjectTopNav
          title="Rust Engineer"
          actions={
            <button
              class="btn btn-sm btn-primary"
              disabled={!canRun() || running()}
              onClick={run}
            >
              <Show when={running()}>
                <span class="loading loading-spinner loading-xs" />
              </Show>
              Generate
            </button>
          }
        />

        <div class="overflow-y-auto min-h-0">
          <div class="px-4 pt-4">
            <div role="tablist" class="tabs tabs-boxed w-fit">
              <For each={MODES}>
                {(m) => (
                  <a
                    role="tab"
                    class={`tab ${mode() === m.value ? 'tab-active' : ''}`}
                    onClick={() => {
                      setMode(m.value);
                      setResult(null);
                      setError(null);
                    }}
                  >
                    {m.label}
                  </a>
                )}
              </For>
            </div>
          </div>

          <Show when={error()}>
            <div class="p-4">
              <div class="alert alert-error alert-sm">
                <span>{error()}</span>
              </div>
            </div>
          </Show>

          <Show when={!running()}>
            <div class="p-4">
              <textarea
                class="textarea textarea-bordered w-full min-h-72 font-mono text-xs"
                placeholder={activeMode().placeholder}
                value={prompt()}
                onInput={(e) => setPrompt(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                    e.preventDefault();
                    void run();
                  }
                }}
              />
            </div>
          </Show>

          <Show when={result()}>
            {(r) => (
              <div class="p-4 flex flex-col gap-4">
                <Show when={r().file_path}>
                  {(fp) => (
                    <div class="alert alert-success alert-sm">
                      <span>
                        Written to <code class="text-xs font-mono">{fp()}</code>
                      </span>
                    </div>
                  )}
                </Show>

                <Show when={r().metadata}>
                  {(meta) => (
                    <div class="alert alert-info alert-sm">
                      <span class="text-xs font-mono">
                        struct: {meta().struct_name ?? '(none)'} · unresolved:{' '}
                        {meta().has_unresolved ? 'yes' : 'no'}
                      </span>
                    </div>
                  )}
                </Show>

                <Show when={r().code}>
                  {(code) => (
                    <div class="card bg-base-200 rounded-lg">
                      <div class="card-body p-4">
                        <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide mb-2">
                          Generated code
                        </h3>
                        <pre class="text-xs font-mono bg-base-300 rounded p-3 overflow-x-auto whitespace-pre-wrap break-words">
                          {code()}
                        </pre>
                      </div>
                    </div>
                  )}
                </Show>

                {/* Transparency panels: prompt + raw model response, per README's
                    "every run returns the prompt, raw model response, and
                    extracted code for debugging" principle. */}
                <details class="card bg-base-200 rounded-lg">
                  <summary class="card-body p-4 cursor-pointer">
                    <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide">
                      Prompt
                    </h3>
                  </summary>
                  <div class="px-4 pb-4">
                    <pre class="text-xs font-mono bg-base-300 rounded p-3 overflow-x-auto whitespace-pre-wrap break-words">
                      {r().prompt}
                    </pre>
                  </div>
                </details>

                <details class="card bg-base-200 rounded-lg">
                  <summary class="card-body p-4 cursor-pointer">
                    <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide">
                      Raw model response
                    </h3>
                  </summary>
                  <div class="px-4 pb-4">
                    <pre class="text-xs font-mono bg-base-300 rounded p-3 overflow-x-auto whitespace-pre-wrap break-words">
                      {r().raw_response}
                    </pre>
                  </div>
                </details>
              </div>
            )}
          </Show>

          <Show when={!result() && !running() && !error()}>
            <div class="flex items-center justify-center h-64 text-base-content/30 text-sm">
              {activeMode().placeholder}
            </div>
          </Show>
        </div>
      </section>
    </main>
  );
}
