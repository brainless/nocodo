import { For, Show, createSignal } from 'solid-js';

export interface PromptBoxProps {
  placeholder?: string;
  examples?: string[];
  submitLabel?: string;
  onSubmit: (value: string) => Promise<void> | void;
}

export function PromptBox(props: PromptBoxProps) {
  const [value, setValue] = createSignal('');
  const [loading, setLoading] = createSignal(false);

  const handleSubmit = async () => {
    const text = value().trim();
    if (!text) return;
    setLoading(true);
    try {
      await props.onSubmit(text);
    } finally {
      setLoading(false);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div class="prompt-box-section">
      <div class="prompt-box">
        <textarea
          class="prompt-box-textarea"
          placeholder={props.placeholder ?? 'What do you want to build?'}
          rows={4}
          value={value()}
          onInput={(e) => setValue(e.currentTarget.value)}
          onKeyDown={handleKeyDown}
          disabled={loading()}
        />
        <div class="prompt-box-footer">
          <span class="prompt-box-hint">
            <kbd class="kbd kbd-sm">⌘</kbd>
            <kbd class="kbd kbd-sm">↵</kbd>
            to send
          </span>
          <button
            class="prompt-box-btn"
            onClick={handleSubmit}
            disabled={loading() || !value().trim()}
          >
            <Show when={loading()}>
              <span class="loading loading-spinner loading-xs" />
            </Show>
            <Show when={!loading()}>
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M5 12h14" /><path d="m12 5 7 7-7 7" />
              </svg>
            </Show>
            {props.submitLabel ?? 'Build it'}
          </button>
        </div>
      </div>

      <Show when={props.examples && props.examples.length > 0}>
        <div class="prompt-box-chips">
          <For each={props.examples}>
            {(ex) => (
              <button
                class="prompt-box-chip"
                onClick={() => setValue(ex)}
                disabled={loading()}
              >
                {ex}
              </button>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
