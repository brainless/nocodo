import { For, Show, createEffect, createMemo, createSignal, onCleanup } from 'solid-js';
import { useProject } from '../contexts/ProjectContext';
import ProjectTopNav from '../components/ProjectTopNav';
import { Sheet, type SheetColumn } from '../components/Sheet';
import type { TaskItem, EpicItem } from '../types/api';

const API_BASE_URL = '';

const STATUS_BADGE: Record<string, string> = {
  open: 'badge-ghost',
  in_progress: 'badge-info',
  review: 'badge-warning',
  done: 'badge-success',
  blocked: 'badge-error',
};

const AGENT_LABEL: Record<string, string> = {
  db_engineer: 'DB Engineer',
  project_manager: 'PM',
  product_owner: 'PO',
  backend_engineer: 'Backend Engineer',
  frontend_engineer: 'Frontend Engineer',
};

function StatusBadge(props: { status: string }) {
  return (
    <span class={`badge badge-sm ${STATUS_BADGE[props.status] ?? 'badge-ghost'}`}>
      {props.status.replace(/_/g, ' ')}
    </span>
  );
}

export default function EpicsPage() {
  const { currentProject, setCurrentProject } = useProject();

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
          const data = await res.json() as { tasks: TaskItem[]; epics: EpicItem[]; updated_at: number; project_name?: string };
          lastUpdatedAt = data.updated_at ?? lastUpdatedAt;
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

  const epicColumns: SheetColumn<EpicItem>[] = [
    {
      key: 'title',
      header: 'Title',
      width: '2fr',
      render: (row) => <span class="truncate text-sm">{row.title}</span>,
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
      render: (row) => <span class="text-sm text-base-content/60">{taskCountByEpic()[row.id] ?? 0}</span>,
    },
    {
      key: 'created_at',
      header: 'Created',
      width: '140px',
      render: (row) => <span class="text-xs text-base-content/50">{new Date(row.created_at * 1000).toLocaleDateString()}</span>,
    },
  ];

  const taskColumns: SheetColumn<TaskItem>[] = [
    {
      key: 'title',
      header: 'Title',
      width: '2fr',
      render: (row) => <span class="truncate text-sm">{row.title}</span>,
    },
    {
      key: 'epic',
      header: 'Epic',
      width: '160px',
      render: (row) => {
        const epic = row.epic_id != null ? epicById()[row.epic_id] : null;
        return <span class="text-xs text-base-content/60 truncate">{epic?.title ?? '—'}</span>;
      },
    },
    {
      key: 'agent',
      header: 'Agent',
      width: '110px',
      render: (row) => <span class="text-xs">{AGENT_LABEL[row.assigned_to_agent] ?? row.assigned_to_agent}</span>,
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
      render: (row) => <span class="text-xs text-base-content/50">{new Date(row.updated_at * 1000).toLocaleDateString()}</span>,
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
    </main>
  );
}
