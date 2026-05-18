import { createContext, useContext, createSignal, type JSX } from 'solid-js';

const API_BASE_URL = '';
const DISPLAY_NAME_KEY = 'nocodo_display_name';

export type UserChatSession = {
  id: number;
  project_id: number;
  status: string;
  created_at: string;
  completed_at?: string;
  handoff_session_id?: number;
};

export type QuestionKind =
  | { type: 'single_choice'; options: string[] }
  | { type: 'multiple_choice'; options: string[] };

export type StructuredQuestion = {
  question: string;
  kind: QuestionKind;
};

export type StructuredResponse = {
  question_message_id: number;
  selected: string[];
};

export type UserChatMessage = {
  id: number;
  session_id: number;
  author_type: 'user' | 'agent' | 'system';
  agent_type?: string;
  content_type: string;
  content: string;
  created_at: string;
};

type GetMessagesData = {
  messages: UserChatMessage[];
  handoff_session_id?: number;
};

export interface UserChatContextValue {
  sessions: () => UserChatSession[];
  messages: () => UserChatMessage[];
  currentSessionId: () => number | null;
  loading: () => boolean;
  displayName: () => string | null;
  handoffSessionId: () => number | null;
  loadSessions: (projectId: number) => Promise<void>;
  startSession: (projectId: number, message: string) => Promise<number | undefined>;
  sendMessage: (sessionId: number, message: string) => Promise<void>;
  sendStructuredResponse: (sessionId: number, response: StructuredResponse) => Promise<void>;
  loadMessages: (sessionId: number) => Promise<void>;
  selectSession: (sessionId: number | null) => Promise<void>;
  setDisplayName: (name: string) => void;
}

const UserChatContext = createContext<UserChatContextValue>();

export function useUserChat(): UserChatContextValue {
  const ctx = useContext(UserChatContext);
  if (!ctx) throw new Error('useUserChat must be used within a UserChatProvider');
  return ctx;
}

export function UserChatProvider(props: { children: JSX.Element }) {
  const [sessions, setSessions] = createSignal<UserChatSession[]>([]);
  const [messages, setMessages] = createSignal<UserChatMessage[]>([]);
  const [currentSessionId, setCurrentSessionId] = createSignal<number | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [handoffSessionId, setHandoffSessionId] = createSignal<number | null>(null);
  const [displayName, setDisplayNameState] = createSignal<string | null>(
    localStorage.getItem(DISPLAY_NAME_KEY)
  );

  const loadSessions = async (projectId: number) => {
    setLoading(true);
    try {
      const res = await fetch(`${API_BASE_URL}/api/user-chats?project_id=${projectId}`);
      if (!res.ok) throw new Error(`Failed to load sessions: ${res.status}`);
      const data = await res.json() as { sessions: UserChatSession[] };
      setSessions(data.sessions ?? []);
    } catch (err) {
      console.error('Error loading sessions:', err);
    } finally {
      setLoading(false);
    }
  };

  const loadMessages = async (sessionId: number) => {
    try {
      const res = await fetch(`${API_BASE_URL}/api/user-chats/${sessionId}/messages`);
      if (!res.ok) throw new Error(`Failed to load messages: ${res.status}`);
      const data = await res.json() as GetMessagesData;
      setMessages(data.messages ?? []);
      if (data.handoff_session_id) {
        setHandoffSessionId(data.handoff_session_id);
      }
    } catch (err) {
      console.error('Error loading messages:', err);
    }
  };

  // Poll messages for the session until it completes or hands off to PM.
  const pollForResponse = async (sessionId: number) => {
    const poll = async () => {
      try {
        const res = await fetch(`${API_BASE_URL}/api/user-chats/${sessionId}/messages`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const data = await res.json() as GetMessagesData;
        setMessages(data.messages ?? []);

        if (data.handoff_session_id) {
          setHandoffSessionId(data.handoff_session_id);
          return; // handoff detected — stop polling
        }

        // Check if session is completed (PM finalized in a planning session).
        const msgs = data.messages ?? [];
        const lastMsg = msgs[msgs.length - 1];
        const sessionCompleted = lastMsg?.author_type === 'agent' &&
          lastMsg?.content_type === 'text';

        // Keep polling if agent hasn't responded yet.
        const hasAgentResponse = (data.messages ?? []).some(m => m.author_type === 'agent');
        if (!hasAgentResponse || (!data.handoff_session_id && !sessionCompleted)) {
          setTimeout(poll, 1500);
        }
      } catch (err) {
        console.error('Error polling messages:', err);
        setTimeout(poll, 2000);
      }
    };

    setTimeout(poll, 1000);
  };

  const startSession = async (projectId: number, message: string): Promise<number | undefined> => {
    const name = displayName() ?? 'User';

    setLoading(true);
    setHandoffSessionId(null);
    try {
      const res = await fetch(`${API_BASE_URL}/api/user-chats`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ project_id: projectId, display_name: name, message }),
      });
      if (!res.ok) throw new Error(`Failed to start session: ${res.status}`);
      const data = await res.json() as { session_id: number; message_id: number };

      setCurrentSessionId(data.session_id);
      setMessages([{
        id: 0,
        session_id: data.session_id,
        author_type: 'user',
        content_type: 'text',
        content: message,
        created_at: new Date().toISOString(),
      }]);

      void pollForResponse(data.session_id);
      await loadSessions(projectId);
      return data.session_id;
    } catch (err) {
      console.error('Error starting session:', err);
    } finally {
      setLoading(false);
    }
  };

  const sendMessage = async (sessionId: number, message: string) => {
    setLoading(true);
    setHandoffSessionId(null);
    setMessages(prev => [...prev, {
      id: 0,
      session_id: sessionId,
      author_type: 'user',
      content_type: 'text',
      content: message,
      created_at: new Date().toISOString(),
    }]);

    try {
      const res = await fetch(`${API_BASE_URL}/api/user-chats/${sessionId}/messages`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ user_id: 1, message }),
      });
      if (!res.ok) throw new Error(`Failed to send message: ${res.status}`);

      void pollForResponse(sessionId);
    } catch (err) {
      console.error('Error sending message:', err);
    } finally {
      setLoading(false);
    }
  };

  const sendStructuredResponse = async (sessionId: number, response: StructuredResponse) => {
    setLoading(true);
    setHandoffSessionId(null);
    setMessages(prev => [...prev, {
      id: 0,
      session_id: sessionId,
      author_type: 'user',
      content_type: 'structured_response',
      content: JSON.stringify(response),
      created_at: new Date().toISOString(),
    }]);

    try {
      const res = await fetch(`${API_BASE_URL}/api/user-chats/${sessionId}/messages`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          user_id: 1,
          message: JSON.stringify(response),
          content_type: 'structured_response',
        }),
      });
      if (!res.ok) throw new Error(`Failed to send response: ${res.status}`);
      void pollForResponse(sessionId);
    } catch (err) {
      console.error('Error sending structured response:', err);
    } finally {
      setLoading(false);
    }
  };

  const selectSession = async (sessionId: number | null) => {
    setCurrentSessionId(sessionId);
    setHandoffSessionId(null);
    if (sessionId !== null) {
      await loadMessages(sessionId);
    } else {
      setMessages([]);
    }
  };

  const setDisplayName = (name: string) => {
    localStorage.setItem(DISPLAY_NAME_KEY, name);
    setDisplayNameState(name);
  };

  const value: UserChatContextValue = {
    sessions,
    messages,
    currentSessionId,
    loading,
    displayName,
    handoffSessionId,
    loadSessions,
    startSession,
    sendMessage,
    sendStructuredResponse,
    loadMessages,
    selectSession,
    setDisplayName,
  };

  return (
    <UserChatContext.Provider value={value}>
      {props.children}
    </UserChatContext.Provider>
  );
}
