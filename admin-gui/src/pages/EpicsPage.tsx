import {
  For,
  Show,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  onCleanup,
} from 'solid-js';
import { useNavigate, useParams } from '@solidjs/router';
import { useProject } from '../contexts/ProjectContext';
import ProjectTopNav from '../components/ProjectTopNav';
import { Sheet, type SheetColumn } from '../components/Sheet';
import type { EpicCommentRow, EpicItem, TaskCommentRow, TaskItem } from '../types/api';

const API_BASE_URL = '';

const STATUS_BADGE: Record<string, string> = {
  open: 'badge-ghost',
  draft: 'badge-ghost',
  in_progress: 'badge-info',
  review: 'badge-warning',
  ready: 'badge-success',
  done: 'badge-success',
  needs_technical_shaping: 'badge-warning',
  blocked: 'badge-error',
};

const AGENT_LABEL: Record<string, string> = {
  db_engineer: 'DB Engineer',
  project_manager: 'PM',
  product_owner: 'PO',
  backend_engineer: 'Backend Engineer',
  frontend_engineer: 'Frontend Engineer',
  ui_designer: 'UI Designer',
  engineering_manager: 'EM',
};

function StatusBadge(props: { status: string }) {
  return (
    <span class={`badge badge-sm ${STATUS_BADGE[props.status] ?? 'badge-ghost'}`}>
      {props.status.replace(/_/g, ' ')}
    </span>
  );
}

function fmtDate(ts: number) {
  return new Date(ts * 1000).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

function fmtDateTime(ts: number) {
  return new Date(ts * 1000).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

// ---------------------------------------------------------------------------
// Detail modal
// ---------------------------------------------------------------------------

type CommentRow = (EpicCommentRow | TaskCommentRow) & { displayName: string };

function DetailModal(props: {
  type: 'epic' | 'task';
  epic?: EpicItem;
  task?: TaskItem;
  epicTitle?: string;
  onClose: () => void;
}) {
  const item = () => (props.type === 'epic' ? props.epic : props.task);
  const itemId = () => item()?.id;

  const fetchComments = async () => {
    const id = itemId();
    if (id == null) return [];
    const url =
      props.type === 'epic'
        ? `${API_BASE_URL}/api/epics/${id}/comments`
        : `${API_BASE_URL}/api/tasks/${id}/comments`;
    const res = await fetch(url);
    if (!res.ok) return [];
    const data = await res.json();
    const rows: CommentRow[] = (
      props.type === 'epic' ? data.comments : data.comments
    ).map((c: EpicCommentRow | TaskCommentRow) => ({
      ...c,
      displayName:
        c.agent_type ? (AGENT_LABEL[c.agent_type] ?? c.agent_type) : 'User',
    }));
    return rows;
  };

  const [comments, { refetch }] = createResource(itemId, fetchComments);
  const [newComment, setNewComment] = createSignal('');
  const [submitting, setSubmitting] = createSignal(false);

  const addComment = async (e: SubmitEvent) => {
    e.preventDefault();
    const text = newComment().trim();
    if (!text) return;
    const id = itemId();
    if (id == null) return;
    setSubmitting(true);
    try {
      const url =
        props.type === 'epic'
          ? `${API_BASE_URL}/api/epics/${id}/comments`
          : `${API_BASE_URL}/api/tasks/${id}/comments`;
      await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ author_type: 'user', content: text }),
      });
      setNewComment('');
      refetch();
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div class="modal modal-open" onClick={props.onClose}>
      <div
        class="modal-box max-w-2xl flex flex-col gap-0 p-0 overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div class="px-6 pt-6 pb-4 border-b border-base-200">
          <div class="flex items-start justify-between gap-3">
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 mb-1">
                <span class="badge badge-sm badge-outline">
                  {props.type === 'epic' ? 'Epic' : 'Task'}
                </span>
                <Show when={props.type === 'task' && props.epicTitle}>
                  <span class="text-xs text-base-content/50 truncate">{props.epicTitle}</span>
                </Show>
              </div>
              <h3 class="text-base font-semibold leading-snug">{item()?.title}</h3>
            </div>
            <button
              class="btn btn-ghost btn-sm btn-circle flex-shrink-0"
              onClick={props.onClose}
              aria-label="Close"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </div>

          {/* Badges row */}
          <div class="flex flex-wrap gap-2 mt-3">
            <Show when={item()}>
              {(it) => <StatusBadge status={it().status} />}
            </Show>
            <Show when={props.type === 'task' && props.task}>
              {(t) => (
                <span class="badge badge-sm badge-ghost">
                  {AGENT_LABEL[t().assigned_to_agent] ?? t().assigned_to_agent}
                </span>
              )}
            </Show>
            <Show when={props.type === 'epic' && props.epic}>
              {(e) => (
                <span class="badge badge-sm badge-ghost">
                  by {AGENT_LABEL[e().created_by_agent] ?? e().created_by_agent}
                </span>
              )}
            </Show>
            <Show when={item()}>
              {(it) => (
                <span class="text-xs text-base-content/40 self-center">
                  Created {fmtDate(it().created_at)}
                </span>
              )}
            </Show>
          </div>
        </div>

        {/* Description / Source prompt */}
        <div class="px-6 py-4 border-b border-base-200">
          <Show when={props.type === 'epic' && props.epic?.description}>
            <p class="text-sm text-base-content/80 whitespace-pre-wrap">{props.epic!.description}</p>
          </Show>
          <Show when={props.type === 'task' && props.task?.source_prompt}>
            <p class="text-sm text-base-content/80 whitespace-pre-wrap">{props.task!.source_prompt}</p>
          </Show>
        </div>

        {/* Comments */}
        <div class="px-6 py-4 flex-1 overflow-y-auto max-h-64">
          <p class="text-xs font-medium text-base-content/50 uppercase tracking-wide mb-3">
            Comments
          </p>

          <Show when={comments.loading}>
            <div class="flex justify-center py-4">
              <span class="loading loading-spinner loading-sm opacity-40" />
            </div>
          </Show>

          <Show when={!comments.loading && (!comments() || comments()!.length === 0)}>
            <p class="text-xs text-base-content/40 italic">No comments yet.</p>
          </Show>

          <For each={comments()}>
            {(comment) => (
              <div class="chat chat-start mb-1">
                <div class="chat-header text-xs opacity-60">
                  {comment.displayName}
                  <time class="ml-2 opacity-50">{fmtDateTime(comment.created_at)}</time>
                </div>
                <div class="chat-bubble chat-bubble-neutral text-sm py-2 px-3">
                  {comment.content}
                </div>
              </div>
            )}
          </For>
        </div>

        {/* Add comment */}
        <form onSubmit={addComment} class="px-6 py-4 border-t border-base-200 flex gap-2">
          <textarea
            class="textarea textarea-bordered textarea-sm flex-1 text-sm resize-none"
            rows={2}
            placeholder="Add a comment…"
            value={newComment()}
            onInput={(e) => setNewComment(e.currentTarget.value)}
            disabled={submitting()}
          />
          <button
            type="submit"
            class="btn btn-primary btn-sm self-end"
            disabled={submitting() || !newComment().trim()}
          >
            {submitting() ? <span class="loading loading-spinner loading-xs" /> : 'Post'}
          </button>
        </form>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export default function EpicsPage() {
  const { currentProject, setCurrentProject } = useProject();
  const params = useParams<{ projectId: string; epicId?: string; taskId?: string }>();
  const navigate = useNavigate();

  const [epics, setEpics] = createSignal<EpicItem[]>([]);
  const [tasks, setTasks] = createSignal<TaskItem[]>([]);
  const [activeTab, setActiveTab] = createSignal<'epics' | 'tasks'>('tasks');
  const [loading, setLoading] = createSignal(false);

  let abortController: AbortController | null = null;

  const startLongPoll = (projectId: number) => {
    abortController?.abort();
    const ac = new AbortController();
    abortController = ac;
    let lastUpdatedAt = 0;

    const poll = async () => {
      while (!ac.signal.aborted) {
        setLoading(true);
        try {
          const url = `${API_BASE_URL}/api/agents/board?project_id=${projectId}&since=${lastUpdatedAt}`;
          const res = await fetch(url, { signal: ac.signal });
          if (!res.ok) {
            await new Promise<void>((r) => setTimeout(r, 3000));
            continue;
          }
          const data = await res.json() as {
            tasks: TaskItem[];
            epics: EpicItem[];
            updated_at: number;
            project_name?: string;
          };
          lastUpdatedAt = data.updated_at || Math.floor(Date.now() / 1000);
          setTasks(data.tasks ?? []);
          setEpics(data.epics ?? []);
          if (data.project_name) {
            const proj = currentProject();
            if (proj && proj.name !== data.project_name) {
              setCurrentProject({ ...proj, name: data.project_name });
            }
          }
        } catch (err) {
          if ((err as Error).name === 'AbortError') break;
          await new Promise<void>((r) => setTimeout(r, 3000));
        } finally {
          setLoading(false);
        }
      }
    };

    poll();
  };

  createEffect(() => {
    const pid = currentProject()?.id;
    if (pid) startLongPoll(pid);
  });

  onCleanup(() => abortController?.abort());

  const epicById = createMemo(() => {
    const map: Record<number, EpicItem> = {};
    for (const e of epics()) map[e.id] = e;
    return map;
  });

  const taskCountByEpic = createMemo(() => {
    const map: Record<number, number> = {};
    for (const t of tasks()) {
      if (t.epic_id != null) map[t.epic_id] = (map[t.epic_id] ?? 0) + 1;
    }
    return map;
  });

  const selectedEpic = createMemo(() => {
    const id = params.epicId ? Number(params.epicId) : null;
    return id != null ? epicById()[id] : undefined;
  });

  const selectedTask = createMemo(() => {
    const id = params.taskId ? Number(params.taskId) : null;
    return id != null ? tasks().find((t) => t.id === id) : undefined;
  });

  const modalType = createMemo<'epic' | 'task' | null>(() => {
    if (params.epicId) return 'epic';
    if (params.taskId) return 'task';
    return null;
  });

  const closeModal = () => {
    navigate(`/projects/${params.projectId}/epics`);
  };

  const epicColumns: SheetColumn<EpicItem>[] = [
    {
      key: 'title',
      header: 'Title',
      width: '2fr',
      render: (row) => (
        <button
          class="truncate text-sm text-left hover:text-primary hover:underline cursor-pointer w-full"
          onClick={() => navigate(`/projects/${params.projectId}/epics/epic/${row.id}`)}
        >
          {row.title}
        </button>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      width: '120px',
      render: (row) => <StatusBadge status={row.status} />,
    },
    {
      key: 'tasks',
      header: 'Tasks',
      width: '80px',
      render: (row) => (
        <span class="text-sm text-base-content/60">{taskCountByEpic()[row.id] ?? 0}</span>
      ),
    },
    {
      key: 'created_at',
      header: 'Created',
      width: '140px',
      render: (row) => (
        <span class="text-xs text-base-content/50">{fmtDate(row.created_at)}</span>
      ),
    },
  ];

  const taskColumns: SheetColumn<TaskItem>[] = [
    {
      key: 'title',
      header: 'Title',
      width: '2fr',
      render: (row) => (
        <button
          class="truncate text-sm text-left hover:text-primary hover:underline cursor-pointer w-full"
          onClick={() => navigate(`/projects/${params.projectId}/epics/task/${row.id}`)}
        >
          {row.title}
        </button>
      ),
    },
    {
      key: 'epic',
      header: 'Epic',
      width: '160px',
      render: (row) => {
        const epic = row.epic_id != null ? epicById()[row.epic_id] : null;
        return (
          <span class="text-xs text-base-content/60 truncate">{epic?.title ?? '—'}</span>
        );
      },
    },
    {
      key: 'agent',
      header: 'Agent',
      width: '110px',
      render: (row) => (
        <span class="text-xs">{AGENT_LABEL[row.assigned_to_agent] ?? row.assigned_to_agent}</span>
      ),
    },
    {
      key: 'status',
      header: 'Status',
      width: '120px',
      render: (row) => <StatusBadge status={row.status} />,
    },
    {
      key: 'updated_at',
      header: 'Updated',
      width: '140px',
      render: (row) => (
        <span class="text-xs text-base-content/50">{fmtDate(row.updated_at)}</span>
      ),
    },
  ];

  return (
    <main class="sheet-app">
      <section class="sheet-main">
        <ProjectTopNav
          title="Epics/Tasks"
          actions={
            <Show when={loading()}>
              <span class="loading loading-spinner loading-xs opacity-40" />
            </Show>
          }
        />

        <Show when={activeTab() === 'epics'}>
          <Sheet
            columns={epicColumns}
            rows={epics()}
            rowKey={(e) => e.id}
            emptyRows={50}
          />
        </Show>

        <Show when={activeTab() === 'tasks'}>
          <Sheet
            columns={taskColumns}
            rows={tasks()}
            rowKey={(t) => t.id}
            emptyRows={50}
          />
        </Show>
      </section>

      <footer class="sheets-strip">
        <div class="tabs tabs-border tabs-sm">
          <button
            class={`tab${activeTab() === 'tasks' ? ' tab-active' : ''}`}
            onClick={() => setActiveTab('tasks')}
          >
            Tasks
            <Show when={tasks().length > 0}>
              <span class="badge badge-sm badge-ghost ml-1">{tasks().length}</span>
            </Show>
          </button>
          <button
            class={`tab${activeTab() === 'epics' ? ' tab-active' : ''}`}
            onClick={() => setActiveTab('epics')}
          >
            Epics
            <Show when={epics().length > 0}>
              <span class="badge badge-sm badge-ghost ml-1">{epics().length}</span>
            </Show>
          </button>
        </div>
      </footer>

      {/* Detail modal — shown when epicId or taskId is in the URL */}
      <Show when={modalType() === 'epic' && selectedEpic()}>
        {(epic) => (
          <DetailModal
            type="epic"
            epic={epic()}
            onClose={closeModal}
          />
        )}
      </Show>

      <Show when={modalType() === 'task' && selectedTask()}>
        {(task) => (
          <DetailModal
            type="task"
            task={task()}
            epicTitle={task().epic_id != null ? epicById()[task().epic_id!]?.title : undefined}
            onClose={closeModal}
          />
        )}
      </Show>
    </main>
  );
}
