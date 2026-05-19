import { Show, createEffect, createSignal, onCleanup } from 'solid-js';
import { useProject } from '../contexts/ProjectContext';
import ProjectTopNav from '../components/ProjectTopNav';
import { Sheet, type SheetColumn } from '../components/Sheet';

type StackNoteRow = {
  id: number;
  project_id: number;
  tag: string;
  note: string;
  file_path: string | null;
  line_number: number | null;
  replaces_id: number | null;
  created_at: number;
  updated_at: number;
};

const TAG_BADGE: Record<string, string> = {
  backend: 'badge-info',
  database: 'badge-warning',
  frontend: 'badge-success',
  auth: 'badge-error',
  api_contract: 'badge-primary',
  config: 'badge-ghost',
  tooling: 'badge-ghost',
  deployment: 'badge-secondary',
  testing: 'badge-accent',
};

function TagBadge(props: { tag: string }) {
  return (
    <span class={`badge badge-sm ${TAG_BADGE[props.tag] ?? 'badge-ghost'}`}>
      {props.tag.replace(/_/g, ' ')}
    </span>
  );
}

const columns: SheetColumn<StackNoteRow>[] = [
  {
    key: 'tag',
    header: 'Tag',
    width: '130px',
    render: (row) => <TagBadge tag={row.tag} />,
  },
  {
    key: 'note',
    header: 'Note',
    width: '3fr',
    render: (row) => <span class="text-sm">{row.note}</span>,
  },
  {
    key: 'file_path',
    header: 'File',
    width: '220px',
    render: (row) => (
      <Show when={row.file_path} fallback={<span class="text-base-content/30">—</span>}>
        <span class="text-xs font-mono text-base-content/60 truncate">
          {row.file_path}
          {row.line_number != null ? `:${row.line_number}` : ''}
        </span>
      </Show>
    ),
  },
  {
    key: 'created_at',
    header: 'Added',
    width: '120px',
    render: (row) => (
      <span class="text-xs text-base-content/50">
        {new Date(row.created_at * 1000).toLocaleDateString()}
      </span>
    ),
  },
];

export default function StackNotesPage() {
  const { currentProject } = useProject();
  const [notes, setNotes] = createSignal<StackNoteRow[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  let abortController: AbortController | null = null;

  const fetchNotes = async (projectId: number) => {
    abortController?.abort();
    const ac = new AbortController();
    abortController = ac;
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/stack-reviewer/notes?project_id=${projectId}`, {
        signal: ac.signal,
      });
      if (!res.ok) {
        setError(`Failed to load notes (${res.status})`);
        return;
      }
      const data = await res.json() as { notes: StackNoteRow[] };
      setNotes(data.notes ?? []);
    } catch (err) {
      if ((err as Error).name !== 'AbortError') {
        setError('Failed to load stack notes');
      }
    } finally {
      setLoading(false);
    }
  };

  createEffect(() => {
    const pid = currentProject()?.id;
    if (pid) fetchNotes(pid);
  });

  onCleanup(() => abortController?.abort());

  return (
    <main class="sheet-app">
      <section class="sheet-main">
        <ProjectTopNav
          title="Tech Stack"
          actions={
            <>
              <button
                class="btn btn-primary btn-sm"
                disabled={loading() || !currentProject()?.id}
                onClick={() => {
                  const pid = currentProject()?.id;
                  if (!pid) return;
                  setLoading(true);
                  fetch('/api/stack-reviewer/run', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ project_id: pid }),
                  })
                    .then(() => fetchNotes(pid))
                    .catch(() => setLoading(false));
                }}
              >
                Review Tech Stack
              </button>
              <Show when={loading()}>
                <span class="loading loading-spinner loading-xs opacity-40" />
              </Show>
            </>
          }
        />

        <Show when={error()}>
          <div class="p-4">
            <div class="alert alert-error alert-sm">
              <span>{error()}</span>
            </div>
          </div>
        </Show>

        <Sheet
          columns={columns}
          rows={notes()}
          rowKey={(n) => n.id}
          emptyRows={50}
        />
      </section>
    </main>
  );
}
