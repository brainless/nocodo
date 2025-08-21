import { Component, createContext, useContext, onMount, onCleanup, ParentComponent } from 'solid-js';
import { createStore } from 'solid-js/store';
import { AiSession, AiSessionStatus } from '../types';
import { apiClient } from '../api';

interface SessionsStore {
  list: AiSession[];
  byId: Record<string, AiSession>;
  loading: boolean;
  error: string | null;
  subscriptions: Record<string, { close: () => void }>;
}

interface SessionsActions {
  fetchList: () => Promise<void>;
  fetchById: (id: string) => Promise<AiSession | null>;
  connect: (id: string) => Promise<void>;
  disconnect: (id: string) => void;
  updateSessionStatus: (id: string, status: AiSessionStatus) => void;
}

interface SessionsContextValue {
  store: SessionsStore;
  actions: SessionsActions;
}

const SessionsContext = createContext<SessionsContextValue>();

export const SessionsProvider: ParentComponent = (props) => {
  const [store, setStore] = createStore<SessionsStore>({
    list: [],
    byId: {},
    loading: false,
    error: null,
    subscriptions: {},
  });

  const actions: SessionsActions = {
    fetchList: async () => {
      try {
        setStore('loading', true);
        setStore('error', null);
        
        const sessions = await apiClient.listSessions();
        
        setStore('list', sessions);
        
        const byId = sessions.reduce((acc, session) => {
          acc[session.id] = session;
          return acc;
        }, {} as Record<string, AiSession>);
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
        const subscription = apiClient.subscribeSession(
          id,
          (data) => {
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
          (error) => {
            console.error(`WebSocket error for session ${id}:`, error);
            setStore('error', `Connection error for session ${id}: ${error.message}`);
          },
          () => {
            console.log(`WebSocket connected for session ${id}`);
          },
          () => {
            console.log(`WebSocket disconnected for session ${id}`);
            setStore('subscriptions', id, undefined!);
          }
        );

        setStore('subscriptions', id, subscription);
        
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to connect to session';
        setStore('error', errorMessage);
        console.error('Failed to connect to session:', err);
      }
    },

    disconnect: (id: string) => {
      const subscription = store.subscriptions[id];
      if (subscription) {
        subscription.close();
        setStore('subscriptions', id, undefined!);
      }
    },

    updateSessionStatus: (id: string, status: AiSessionStatus) => {
      setStore('byId', id, 'status', status);
      
      const existingIndex = store.list.findIndex(s => s.id === id);
      if (existingIndex >= 0) {
        setStore('list', existingIndex, 'status', status);
      }
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

  return (
    <SessionsContext.Provider value={contextValue}>
      {props.children}
    </SessionsContext.Provider>
  );
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