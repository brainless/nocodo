import { ParentComponent, createContext, onCleanup, useContext } from 'solid-js';
import { createStore } from 'solid-js/store';
import { AiSessionStatus, ExtendedAiSession } from '../types';
import { apiClient } from '../api';

interface OutputChunk {
  stream?: 'stdout' | 'stderr';
  content: string;
  seq?: number;
  created_at?: number;
}

interface SessionsStore {
  list: ExtendedAiSession[];
  byId: Record<string, ExtendedAiSession>;
  loading: boolean;
  error: string | null;
  subscriptions: Record<string, { close: () => void }>;
  pollingTimers: Record<string, NodeJS.Timeout>;
  connectionStatus: Record<string, 'connected' | 'disconnected' | 'error' | 'fallback'>;
  outputsBySession: Record<string, { chunks: OutputChunk[]; lastSeq?: number }>;
}

interface SessionsActions {
  fetchList: () => Promise<void>;
  fetchById: (id: string) => Promise<ExtendedAiSession | null>;
  fetchOutputs: (id: string) => Promise<void>;
  connect: (id: string) => Promise<void>;
  disconnect: (id: string) => void;
  updateSessionStatus: (id: string, status: AiSessionStatus) => void;
  startPolling: (id: string) => void;
  stopPolling: (id: string) => void;
  getConnectionStatus: (id: string) => 'connected' | 'disconnected' | 'error' | 'fallback';
  appendOutputChunk: (id: string, chunk: OutputChunk) => void;
  clearOutputs: (id: string) => void;
  getOutputs: (id: string) => OutputChunk[];
}

interface SessionsContextValue {
  store: SessionsStore;
  actions: SessionsActions;
}

const SessionsContext = createContext<SessionsContextValue>();

export const SessionsProvider: ParentComponent = props => {
  const [store, setStore] = createStore<SessionsStore>({
    list: [],
    byId: {},
    loading: false,
    error: null,
    subscriptions: {},
    pollingTimers: {},
    connectionStatus: {},
    outputsBySession: {},
  });

  const actions: SessionsActions = {
    fetchList: async () => {
      try {
        setStore('loading', true);
        setStore('error', null);

        const sessions = await apiClient.listSessions();

        // Defensive check: ensure sessions is an array
        const sessionsList = Array.isArray(sessions) ? sessions : [];
        setStore('list', sessionsList);

        const byId = sessionsList.reduce(
          (acc, session) => {
            acc[session.id] = session;
            return acc;
          },
          {} as Record<string, ExtendedAiSession>
        );
        setStore('byId', byId);
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to fetch sessions';
        setStore('error', errorMessage);
        console.error('Failed to fetch sessions:', err);
      } finally {
        setStore('loading', false);
      }
    },

    fetchById: async (id: string) => {
      try {
        setStore('error', null);

        const session = await apiClient.getSession(id);

        setStore('byId', id, session);

        const existingIndex = store.list.findIndex(s => s.id === id);
        if (existingIndex >= 0) {
          setStore('list', existingIndex, session);
        } else {
          setStore('list', prev => [...prev, session]);
        }

        return session;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to fetch session';
        setStore('error', errorMessage);
        console.error('Failed to fetch session:', err);
        return null;
      }
    },

    fetchOutputs: async (id: string) => {
      try {
        console.debug(`Fetching outputs for work ${id}`);
        const listResp = await apiClient.listAiOutputs(id);
        const chunks = (listResp.outputs || []).map((o, idx) => ({
          content: o.content,
          created_at: o.created_at,
          seq: idx,
          stream: 'stdout' as const,
        }));
        setStore('outputsBySession', id, {
          chunks,
          lastSeq: chunks.length ? chunks.length - 1 : 0,
        });
        console.debug(`Loaded ${chunks.length} output chunks for work ${id}`);
      } catch (err) {
        console.warn(`Outputs not available for work ${id}:`, err);
        // initialize empty container to avoid undefined checks
        if (!store.outputsBySession[id]) {
          setStore('outputsBySession', id, { chunks: [], lastSeq: 0 });
        }
      }
    },

    connect: async (id: string) => {
      if (store.subscriptions[id]) {
        console.warn(`Already connected to session ${id}`);
        return;
      }

      try {
        setStore('connectionStatus', id, 'connected');

        const subscription = apiClient.subscribeSession(
          id,
          data => {
            // WebSocket is working, stop any fallback polling
            actions.stopPolling(id);
            setStore('connectionStatus', id, 'connected');

            if (
              (data.type === 'status_update' || data.type === 'AiSessionStatusChanged') &&
              (data.status || data.payload?.status)
            ) {
              const newStatus = data.status ?? data.payload?.status;
              actions.updateSessionStatus(id, newStatus);
            }
            if (data.type === 'session_data' && data.session) {
              setStore('byId', id, data.session);
              const existingIndex = store.list.findIndex(s => s.id === id);
              if (existingIndex >= 0) {
                setStore('list', existingIndex, data.session);
              }
            }
            if (data.type === 'AiSessionOutputChunk' && data.payload) {
              const payload = data.payload as {
                session_id: string;
                stream?: 'stdout' | 'stderr';
                content: string;
                seq?: number;
              };
              if (payload.session_id === id) {
                actions.appendOutputChunk(id, {
                  stream: payload.stream,
                  content: payload.content,
                  seq: payload.seq,
                });
              }
            }
            if (data.type === 'LlmAgentChunk' && data.payload) {
              const payload = data.payload as {
                session_id: string;
                content: string;
              };
              if (payload.session_id === id) {
                actions.appendOutputChunk(id, {
                  stream: 'stdout',
                  content: payload.content,
                });
              }
            }
          },
          error => {
            console.error(`WebSocket error for session ${id}:`, error);
            setStore('connectionStatus', id, 'error');

            // Start polling as fallback
            console.log(`Starting polling fallback for session ${id}`);
            actions.startPolling(id);
            setStore('connectionStatus', id, 'fallback');
          },
          () => {
            console.log(`WebSocket connected for session ${id}`);
            setStore('connectionStatus', id, 'connected');
          },
          () => {
            console.log(`WebSocket disconnected for session ${id}`);
            setStore('subscriptions', id, undefined!);
            setStore('connectionStatus', id, 'disconnected');

            // Start polling as fallback if session is still running
            const session = store.byId[id];
            if (session && session.status === 'running') {
              console.log(`Starting polling fallback for disconnected session ${id}`);
              actions.startPolling(id);
              setStore('connectionStatus', id, 'fallback');
            }
          }
        );

        setStore('subscriptions', id, subscription);
      } catch (err) {
        // const errorMessage = err instanceof Error ? err.message : 'Failed to connect to session';
        console.error('Failed to connect to session:', err);
        setStore('connectionStatus', id, 'error');

        // Start polling as fallback
        actions.startPolling(id);
        setStore('connectionStatus', id, 'fallback');
      }
    },

    disconnect: (id: string) => {
      try {
        const subscription = store.subscriptions[id];
        if (subscription) {
          subscription.close();
          setStore('subscriptions', id, undefined!);
        }

        // Stop any polling timers
        actions.stopPolling(id);

        // Only update connection status if store still exists
        if (store.connectionStatus) {
          setStore('connectionStatus', id, 'disconnected');
        }
      } catch (error) {
        // Ignore errors during cleanup (component might be unmounting)
        console.warn(`Error during session disconnect for ${id}:`, error);
      }
    },

    appendOutputChunk: (id: string, chunk: OutputChunk) => {
      const existing = store.outputsBySession[id]?.chunks || [];
      // Avoid duplicates by seq if provided
      if (chunk.seq !== undefined) {
        const exists = existing.some(c => c.seq === chunk.seq);
        if (exists) return;
      }
      const newChunks = [...existing, chunk].sort((a, b) => (a.seq ?? 0) - (b.seq ?? 0));
      const lastSeq = newChunks.length
        ? newChunks[newChunks.length - 1].seq
        : store.outputsBySession[id]?.lastSeq;
      setStore('outputsBySession', id, { chunks: newChunks, lastSeq });
    },

    clearOutputs: (id: string) => {
      setStore('outputsBySession', id, { chunks: [], lastSeq: 0 });
    },

    updateSessionStatus: (id: string, status: AiSessionStatus) => {
      setStore('byId', id, 'status', status);

      const existingIndex = store.list.findIndex(s => s.id === id);
      if (existingIndex >= 0) {
        setStore('list', existingIndex, 'status', status);
      }

      // Stop polling if session is no longer running
      if (status !== 'running') {
        actions.stopPolling(id);
      }
    },

    startPolling: (id: string) => {
      // Don't start polling if already polling
      if (store.pollingTimers[id]) {
        return;
      }

      const pollSession = async () => {
        try {
          const session = await apiClient.getSession(id);
          if (session) {
            // Update the session data
            setStore('byId', id, session);
            const existingIndex = store.list.findIndex(s => s.id === id);
            if (existingIndex >= 0) {
              setStore('list', existingIndex, session);
            }

            // Stop polling if session is no longer running
            if (session.status !== 'running') {
              actions.stopPolling(id);
            }
          }
        } catch (err) {
          console.error(`Polling error for session ${id}:`, err);
          // Continue polling even on errors
        }
      };

      // Poll immediately, then every 5 seconds
      pollSession();
      const timerId = setInterval(pollSession, 5000);
      setStore('pollingTimers', id, timerId);

      console.log(`Started polling for session ${id}`);
    },

    stopPolling: (id: string) => {
      try {
        const timerId = store.pollingTimers?.[id];
        if (timerId) {
          clearInterval(timerId);
          if (store.pollingTimers) {
            setStore('pollingTimers', id, undefined!);
          }
          console.log(`Stopped polling for session ${id}`);
        }
      } catch (error) {
        // Ignore errors during cleanup
        console.warn(`Error stopping polling for session ${id}:`, error);
      }
    },

    getConnectionStatus: (id: string) => {
      return store.connectionStatus[id] || 'disconnected';
    },

    getOutputs: (id: string) => {
      return store.outputsBySession[id]?.chunks || [];
    },
  };

  onCleanup(() => {
    Object.values(store.subscriptions).forEach(subscription => {
      if (subscription) {
        subscription.close();
      }
    });
  });

  const contextValue: SessionsContextValue = {
    store,
    actions,
  };

  return <SessionsContext.Provider value={contextValue}>{props.children}</SessionsContext.Provider>;
};

export const useSessions = () => {
  const context = useContext(SessionsContext);
  if (!context) {
    throw new Error('useSessions must be used within a SessionsProvider');
  }
  return context;
};

export const useSessionsList = () => {
  const { store } = useSessions();
  return {
    sessions: store.list,
    loading: store.loading,
    error: store.error,
  };
};

export const useSession = (id: string) => {
  const { store } = useSessions();
  return {
    session: store.byId[id] || null,
    loading: store.loading,
    error: store.error,
  };
};

export const useSessionOutputs = (id: string) => {
  const { actions } = useSessions();
  return {
    get: () => actions.getOutputs(id),
  };
};
