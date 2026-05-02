import { createContext, useContext, createSignal, createEffect, type JSX } from 'solid-js';
import type { SessionItem, ListSessionsResponse } from '../types/api';

const API_BASE_URL = '';

export type ChatRole = 'user' | 'assistant';
export type UiMessage = { role: ChatRole; content: string; schema_version?: number };

type HistoryMessage = {
  id: number;
  role: string;
  content: string;
  created_at: number;
  schema_version?: number;
};

const DEFAULT_GREETING: UiMessage = {
  role: 'assistant',
  content: "Hello! Tell me what you want to build and I'll design a schema for it.",
};

export interface ChatContextValue {
  messages: () => UiMessage[];
  chatLoading: () => boolean;
  sessions: () => SessionItem[];
  sessionsLoading: () => boolean;
  selectedSession: () => SessionItem | null;
  sendMessage: (text: string) => Promise<void>;
  selectSession: (session: SessionItem) => Promise<void>;
  loadSessions: () => Promise<void>;
}

const ChatContext = createContext<ChatContextValue>();

export function useChat(): ChatContextValue {
  const ctx = useContext(ChatContext);
  if (!ctx) throw new Error('useChat must be used within a ChatProvider');
  return ctx;
}

interface ChatProviderProps {
  agentType: string;
  projectId: () => number | null | undefined;
  greeting?: UiMessage;
  children: JSX.Element;
}

export function ChatProvider(props: ChatProviderProps) {
  const agentApiPath = () => props.agentType.replace(/_/g, '-');

  const [messages, setMessages] = createSignal<UiMessage[]>([
    props.greeting ?? DEFAULT_GREETING,
  ]);
  const [chatLoading, setChatLoading] = createSignal(false);
  const [sessions, setSessions] = createSignal<SessionItem[]>([]);
  const [sessionsLoading, setSessionsLoading] = createSignal(false);
  const [selectedSession, setSelectedSession] = createSignal<SessionItem | null>(null);

  const loadSessions = async (projectId?: number, agentTypeArg?: string) => {
    const pid = projectId ?? props.projectId();
    if (!pid) return;
    setSessionsLoading(true);
    try {
      const type = agentTypeArg ?? props.agentType;
      const url = `${API_BASE_URL}/api/agents/sessions?project_id=${pid}&agent_type=${type}`;
      const response = await fetch(url);
      if (!response.ok) throw new Error(`Failed to load sessions: ${response.status}`);
      const data = await response.json() as ListSessionsResponse;
      setSessions(data.sessions);
      if (data.sessions.length > 0) {
        const latest = data.sessions.sort((a, b) => b.created_at - a.created_at)[0];
        await selectSession(latest);
      } else {
        setSelectedSession(null);
      }
    } catch (error) {
      console.error('Error loading sessions:', error);
    } finally {
      setSessionsLoading(false);
    }
  };

  const selectSession = async (session: SessionItem) => {
    setSelectedSession(session);
    setMessages([props.greeting ?? DEFAULT_GREETING]);
    setChatLoading(true);
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/agents/${agentApiPath()}/sessions/${session.id}/messages`
      );
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json() as { messages?: HistoryMessage[] };
      const history = (data.messages ?? [])
        .filter((m) => m.role === 'user' || m.role === 'assistant')
        .map((m) => ({
          role: m.role as ChatRole,
          content: m.content,
          schema_version: m.schema_version,
        }));
      if (history.length > 0) setMessages(history);
    } catch (error) {
      console.error('Error loading session history:', error);
    } finally {
      setChatLoading(false);
    }
  };

  const pollForResponse = async (messageId: number, sessionId: number) => {
    try {
      const response = await fetch(
        `${API_BASE_URL}/api/agents/${agentApiPath()}/messages/${messageId}/response`
      );
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json();
      if (data.response?.type === 'pending') {
        setTimeout(() => pollForResponse(messageId, sessionId), 500);
        return;
      }
      const histRes = await fetch(
        `${API_BASE_URL}/api/agents/${agentApiPath()}/sessions/${sessionId}/messages`
      );
      if (histRes.ok) {
        const histData = await histRes.json() as { messages?: HistoryMessage[] };
        const history = (histData.messages ?? [])
          .filter((m) => m.role === 'user' || m.role === 'assistant')
          .map((m) => ({
            role: m.role as ChatRole,
            content: m.content,
            schema_version: m.schema_version,
          }));
        if (history.length > 0) setMessages(history);
      }
    } catch (error) {
      console.error('Error polling response:', error);
      setMessages((prev) => [
        ...prev,
        {
          role: 'assistant',
          content: `Error: ${error instanceof Error ? error.message : 'Unknown error'}`,
        },
      ]);
    } finally {
      setChatLoading(false);
    }
  };

  const sendMessage = async (message: string) => {
    const session = selectedSession();
    const projectId = props.projectId();
    if (!message || !projectId) return;

    setMessages((prev) => [...prev, { role: 'user', content: message }]);
    setChatLoading(true);

    try {
      const body: Record<string, unknown> = { project_id: projectId, message };
      if (session) body.session_id = session.id;

      const response = await fetch(`${API_BASE_URL}/api/agents/${agentApiPath()}/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json() as { session_id: number; message_id: number };

      if (!session && data.session_id) {
        setSelectedSession({
          id: data.session_id,
          project_id: projectId,
          agent_type: props.agentType,
          created_at: Math.floor(Date.now() / 1000),
        });
      }

      if (data.message_id) {
        pollForResponse(data.message_id, data.session_id ?? session!.id);
      } else {
        setChatLoading(false);
      }
    } catch (error) {
      console.error('Error sending message:', error);
      setChatLoading(false);
      setMessages((prev) => [
        ...prev,
        {
          role: 'assistant',
          content: `Error: ${error instanceof Error ? error.message : 'Failed to send'}`,
        },
      ]);
    }
  };

  createEffect(() => {
    const pid = props.projectId();
    if (pid) loadSessions(pid);
  });

  const value: ChatContextValue = {
    messages,
    chatLoading,
    sessions,
    sessionsLoading,
    selectedSession,
    sendMessage,
    selectSession,
    loadSessions: () => loadSessions(),
  };

  return <ChatContext.Provider value={value}>{props.children}</ChatContext.Provider>;
}
