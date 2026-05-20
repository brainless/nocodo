import { Show, createEffect, createSignal, onCleanup } from 'solid-js';
import { useProject } from '../contexts/ProjectContext';
import ProjectTopNav from '../components/ProjectTopNav';
import { Sheet, type SheetColumn } from '../components/Sheet';

type ProjectNoteRow = {
  id: number;
  project_id: number;
  topic: string;
  note: string;
  source_session_id: number | null;
  source_epic_comment_id: number | null;
  source_task_comment_id: number | null;
  replaces_id: number | null;
  created_at: number;
};

const TOPIC_BADGE: Record<string, string> = {
  goal: 'badge-primary',
  constraint: 'badge-error',
  decision: 'badge-warning',
  context: 'badge-info',
  assumption: 'badge-ghost',
};

function TopicBadge(props: { topic: string }) {
  return (
    <span class={`badge badge-sm ${TOPIC_BADGE[props.topic] ?? 'badge-ghost'}`}>
      {props.topic}
    </span>
  );
}

const columns: SheetColumn<ProjectNoteRow>[] = [
  {
    key: 'topic',
    header: 'Topic',
    width: '120px',
    render: (row) => <TopicBadge topic={row.topic} />,
  },
  {
    key: 'note',
    header: 'Note',
    width: '3fr',
    render: (row) => <span class="text-sm">{row.note}</span>,
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

export default function ProjectNotesPage() {
  const { currentProject } = useProject();
  const [notes, setNotes] = createSignal<ProjectNoteRow[]>([]);
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
      const res = await fetch(`/api/project-notes?project_id=${projectId}`, {
        signal: ac.signal,
      });
      if (!res.ok) {
        setError(`Failed to load notes (${res.status})`);
        return;
      }
      const data = await res.json() as { notes: ProjectNoteRow[] };
      setNotes(data.notes ?? []);
    } catch (err) {
      if ((err as Error).name !== 'AbortError') {
        setError('Failed to load project notes');
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
          title="Project Notes"
          actions={
            <Show when={loading()}>
              <span class="loading loading-spinner loading-xs opacity-40" />
            </Show>
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
