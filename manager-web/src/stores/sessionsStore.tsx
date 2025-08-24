import { ParentComponent, createContext, onCleanup, useContext } from 'solid-js';
import { createStore } from 'solid-js/store';
import { AiSession, AiSessionStatus } from '../types';
import { apiClient } from '../api';

interface SessionsStore {
  list: AiSession[];
  byId: Record<string, AiSession>;
  loading: boolean;
  error: string | null;
  subscriptions: Record<string, { close: () => void }>;
  pollingTimers: Record<string, NodeJS.Timeout>;
  connectionStatus: Record<string, 'connected' | 'disconnected' | 'error' | 'fallback'>;
}

interface SessionsActions {
  fetchList: () => Promise<void>;
  fetchById: (id: string) => Promise<AiSession | null>;
  connect: (id: string) => Promise<void>;
  disconnect: (id: string) => void;
  updateSessionStatus: (id: string, status: AiSessionStatus) => void;
  startPolling: (id: string) => void;
  stopPolling: (id: string) => void;
  getConnectionStatus: (id: string) => 'connected' | 'disconnected' | 'error' | 'fallback';
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
  });

  const actions: SessionsActions = {
    fetchList: async () => {
      try {
        setStore('loading', true);
        setStore('error', null);

        const sessions = await apiClient.listSessions();

        setStore('list', sessions);

        const byId = sessions.reduce(
          (acc, session) => {
            acc[session.id] = session;
            return acc;
          },
          {} as Record<string, AiSession>
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

            if (data.type === 'status_update' && data.status) {
              actions.updateSessionStatus(id, data.status);
            }
            if (data.type === 'session_data' && data.session) {
              setStore('byId', id, data.session);
              const existingIndex = store.list.findIndex(s => s.id === id);
              if (existingIndex >= 0) {
                setStore('list', existingIndex, data.session);
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
