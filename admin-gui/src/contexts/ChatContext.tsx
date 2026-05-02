import { createContext, useContext, createSignal, createEffect, type JSX } from 'solid-js';
import type { TaskItem } from '../types/api';

const API_BASE_URL = '';

export type ChatRole = 'user' | 'assistant';
export type UiMessage = { role: ChatRole; content: string; schema_version?: number };

type HistoryMessage = {
  id: number;
  role: string;
  content: string;
  created_at: number;
  schema_version?: number;
  tool_name?: string;
};

export interface ChatContextValue {
  messages: () => UiMessage[];
  chatLoading: () => boolean;
  tasks: () => TaskItem[];
  tasksLoading: () => boolean;
  selectedTask: () => TaskItem | null;
  sendMessage: (text: string) => Promise<void>;
  openTask: (taskId: number, agentType: string) => Promise<void>;
  loadTasks: () => Promise<void>;
}

const ChatContext = createContext<ChatContextValue>();

export function useChat(): ChatContextValue {
  const ctx = useContext(ChatContext);
  if (!ctx) throw new Error('useChat must be used within a ChatProvider');
  return ctx;
}

interface ChatProviderProps {
  agentType?: string;
  projectId: () => number | null | undefined;
  greeting?: UiMessage;
  children: JSX.Element;
}

const DEFAULT_GREETING: UiMessage = {
  role: 'assistant',
  content: "Hello! Tell me what you want to build and I'll design a schema for it.",
};

export function ChatProvider(props: ChatProviderProps) {
  const [messages, setMessages] = createSignal<UiMessage[]>([props.greeting ?? DEFAULT_GREETING]);
  const [chatLoading, setChatLoading] = createSignal(false);
  const [tasks, setTasks] = createSignal<TaskItem[]>([]);
  const [tasksLoading, setTasksLoading] = createSignal(false);
  const [selectedTask, setSelectedTask] = createSignal<TaskItem | null>(null);
  // Track which agent type is active for the current conversation
  const [activeAgentType, setActiveAgentType] = createSignal<string>(props.agentType ?? '');

  const agentPath = () => activeAgentType().replace(/_/g, '-');

  const loadTasks = async () => {
    const pid = props.projectId();
    if (!pid) return;
    setTasksLoading(true);
    try {
      const url = `${API_BASE_URL}/api/agents/tasks?project_id=${pid}`;
      const res = await fetch(url);
      if (!res.ok) throw new Error(`Failed to load tasks: ${res.status}`);
      const data = await res.json() as { tasks: TaskItem[] };
      const all = data.tasks ?? [];
      // If agentType is set on the provider, filter to that agent's tasks for the sidebar
      const filtered = props.agentType
        ? all.filter(t => t.assigned_to_agent === props.agentType)
        : all;
      setTasks(filtered);
      // Auto-select most recent task for this agent if none selected
      if (!selectedTask() && filtered.length > 0) {
        const latest = filtered.slice().sort((a, b) => b.created_at - a.created_at)[0];
        await openTask(latest.id, latest.assigned_to_agent);
      }
    } catch (err) {
      console.error('Error loading tasks:', err);
    } finally {
      setTasksLoading(false);
    }
  };

  const openTask = async (taskId: number, agentType: string) => {
    setActiveAgentType(agentType);
    const path = agentType.replace(/_/g, '-');
    setChatLoading(true);
    try {
      const res = await fetch(`${API_BASE_URL}/api/agents/${path}/tasks/${taskId}/messages`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json() as { task_id: number; messages: HistoryMessage[] };

      const found = tasks().find(t => t.id === taskId);
      if (found) {
        setSelectedTask(found);
      } else {
        // Task might not be in filtered list (e.g. task board click across agents)
        setSelectedTask({ id: taskId, assigned_to_agent: agentType } as TaskItem);
      }

      const history = (data.messages ?? [])
        .filter(m => m.role === 'user' || m.role === 'assistant')
        .map(m => ({ role: m.role as ChatRole, content: m.content, schema_version: m.schema_version }));

      setMessages(history.length > 0 ? history : [props.greeting ?? DEFAULT_GREETING]);
    } catch (err) {
      console.error('Error opening task:', err);
      setMessages([props.greeting ?? DEFAULT_GREETING]);
    } finally {
      setChatLoading(false);
    }
  };

  const pollForResponse = async (messageId: number, taskId: number) => {
    try {
      const res = await fetch(`${API_BASE_URL}/api/agents/${agentPath()}/messages/${messageId}/response`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      if (data.response?.type === 'pending') {
        setTimeout(() => pollForResponse(messageId, taskId), 500);
        return;
      }
      // Reload messages from server
      const histRes = await fetch(`${API_BASE_URL}/api/agents/${agentPath()}/tasks/${taskId}/messages`);
      if (histRes.ok) {
        const histData = await histRes.json() as { messages: HistoryMessage[] };
        const history = (histData.messages ?? [])
          .filter(m => m.role === 'user' || m.role === 'assistant')
          .map(m => ({ role: m.role as ChatRole, content: m.content, schema_version: m.schema_version }));
        if (history.length > 0) setMessages(history);
      }
      // Refresh task list so status changes are reflected
      await loadTasks();
    } catch (err) {
      console.error('Error polling response:', err);
      setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${err instanceof Error ? err.message : 'Unknown error'}` }]);
    } finally {
      setChatLoading(false);
    }
  };

  const sendMessage = async (message: string) => {
    const pid = props.projectId();
    if (!message || !pid) return;

    const currentTask = selectedTask();
    const agent = activeAgentType() || props.agentType || '';
    if (!agent) return;

    setMessages(prev => [...prev, { role: 'user', content: message }]);
    setChatLoading(true);

    try {
      const body: Record<string, unknown> = { project_id: pid, message };
      if (currentTask) body.task_id = currentTask.id;

      const res = await fetch(`${API_BASE_URL}/api/agents/${agentPath()}/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json() as { task_id: number; message_id: number };

      // If this was a new task, update selected task
      if (!currentTask && data.task_id) {
        const newTask: TaskItem = {
          id: data.task_id,
          project_id: pid,
          epic_id: null,
          title: message.slice(0, 100),
          source_prompt: message,
          assigned_to_agent: agent,
          status: 'in_progress',
          created_at: Math.floor(Date.now() / 1000),
          updated_at: Math.floor(Date.now() / 1000),
        };
        setSelectedTask(newTask);
        setTasks(prev => [...prev, newTask]);
      }

      if (data.message_id) {
        pollForResponse(data.message_id, data.task_id);
      } else {
        setChatLoading(false);
      }
    } catch (err) {
      console.error('Error sending message:', err);
      setChatLoading(false);
      setMessages(prev => [...prev, { role: 'assistant', content: `Error: ${err instanceof Error ? err.message : 'Failed to send'}` }]);
    }
  };

  createEffect(() => {
    const pid = props.projectId();
    if (pid) loadTasks();
  });

  const value: ChatContextValue = {
    messages,
    chatLoading,
    tasks,
    tasksLoading,
    selectedTask,
    sendMessage,
    openTask,
    loadTasks,
  };

  return <ChatContext.Provider value={value}>{props.children}</ChatContext.Provider>;
}
