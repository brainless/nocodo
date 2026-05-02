import { For, Show, createEffect, createMemo, createSignal, onCleanup } from 'solid-js';
import { useProject } from '../contexts/ProjectContext';
import { ChatProvider, useChat } from '../contexts/ChatContext';
import ChatDrawer from '../components/ChatDrawer';
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
  schema_designer: 'DB Dev',
  project_manager: 'PM',
  backend_developer: 'Backend Dev',
  frontend_developer: 'Frontend Dev',
};

function StatusBadge(props: { status: string }) {
  return (
    <span class={`badge badge-sm ${STATUS_BADGE[props.status] ?? 'badge-ghost'}`}>
      {props.status.replace('_', ' ')}
    </span>
  );
}

export default function ProjectManagerPage() {
  const { currentProject } = useProject();

  return (
    <ChatProvider
      agentType="project_manager"
      projectId={() => currentProject()?.id}
      greeting={{ role: 'assistant', content: "Hi! Tell me what you want to build and I'll plan the epics and tasks." }}
    >
      <ProjectManagerContent />
    </ChatProvider>
  );
}

function ProjectManagerContent() {
  const { currentProject } = useProject();
  const chat = useChat();

  const [epics, setEpics] = createSignal<EpicItem[]>([]);
  const [tasks, setTasks] = createSignal<TaskItem[]>([]);
  const [activeTab, setActiveTab] = createSignal<'epics' | 'tasks'>('tasks');
  const [loading, setLoading] = createSignal(false);

  const loadBoard = async (projectId: number) => {
    setLoading(true);
    try {
      const [epicsRes, tasksRes] = await Promise.all([
        fetch(`${API_BASE_URL}/api/agents/epics?project_id=${projectId}`),
        fetch(`${API_BASE_URL}/api/agents/tasks?project_id=${projectId}`),
      ]);
      if (epicsRes.ok) {
        const d = await epicsRes.json() as { epics: EpicItem[] };
        setEpics(d.epics ?? []);
      }
      if (tasksRes.ok) {
        const d = await tasksRes.json() as { tasks: TaskItem[] };
        setTasks(d.tasks ?? []);
      }
    } finally {
      setLoading(false);
    }
  };

  createEffect(() => {
    const pid = currentProject()?.id;
    if (pid) loadBoard(pid);
  });

  // Poll every 5s to pick up agent status changes
  const interval = setInterval(() => {
    const pid = currentProject()?.id;
    if (pid) loadBoard(pid);
  }, 5000);
  onCleanup(() => clearInterval(interval));

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

  const handleEpicClick = (epic: EpicItem) => {
    if (epic.created_by_task_id != null) {
      const drawer = document.getElementById('chat-drawer') as HTMLInputElement | null;
      if (drawer) drawer.checked = true;
      chat.openTask(epic.created_by_task_id, 'project_manager');
    }
  };

  const handleTaskClick = (task: TaskItem) => {
    const drawer = document.getElementById('chat-drawer') as HTMLInputElement | null;
    if (drawer) drawer.checked = true;
    chat.openTask(task.id, task.assigned_to_agent);
  };

  const placeholder = () =>
    chat.messages().length > 1
      ? 'Ask the PM to update or add to the plan...'
      : 'Describe what you want to build...';

  return (
    <main class="sheet-app">
      <ChatDrawer agentName="Project Manager" placeholder={placeholder}>
        <section class="sheet-main">
          <div class="formula-strip">
            <label for="chat-drawer" class="btn btn-success btn-sm">Dev Team</label>
            <Show when={loading()}>
              <span class="loading loading-spinner loading-xs ml-2 opacity-40" />
            </Show>
          </div>

          <Show when={activeTab() === 'epics'}>
            <Sheet
              columns={epicColumns}
              rows={epics()}
              rowKey={(e) => e.id}
              onRowClick={handleEpicClick}
              emptyRows={50}
            />
          </Show>

          <Show when={activeTab() === 'tasks'}>
            <Sheet
              columns={taskColumns}
              rows={tasks()}
              rowKey={(t) => t.id}
              onRowClick={handleTaskClick}
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
      </ChatDrawer>
    </main>
  );
}
