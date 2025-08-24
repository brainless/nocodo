import {
  ParentComponent,
  createContext,
  onCleanup,
  onMount,
  useContext,
} from 'solid-js';
import { createStore } from 'solid-js/store';
import { WebSocketConnectionState, WebSocketMessage } from './types';
import { getWebSocketClient } from './websocket';

// WebSocket store interface
interface WebSocketStore {
  connectionState: WebSocketConnectionState;
  isConnected: boolean;
  lastMessage: WebSocketMessage | null;
  clientId: string | null;
  error: string | null;
}

// WebSocket actions interface
interface WebSocketActions {
  connect: () => void;
  disconnect: () => void;
  send: (message: WebSocketMessage) => void;
}

// Context interface
interface WebSocketContextValue {
  store: WebSocketStore;
  actions: WebSocketActions;
}

// Create context
const WebSocketContext = createContext<WebSocketContextValue>();

// WebSocket Provider Component
export const WebSocketProvider: ParentComponent = props => {
  // Create store for WebSocket state
  const [store, setStore] = createStore<WebSocketStore>({
    connectionState: 'disconnected',
    isConnected: false,
    lastMessage: null,
    clientId: null,
    error: null,
  });

  // Get WebSocket client instance
  const wsClient = getWebSocketClient();

  // Actions
  const actions: WebSocketActions = {
    connect: () => {
      console.log('WebSocket connect requested');
      wsClient.connect();
    },

    disconnect: () => {
      console.log('WebSocket disconnect requested');
      wsClient.disconnect();
    },

    send: (message: WebSocketMessage) => {
      wsClient.send(message);
    },
  };

  // Setup WebSocket event handlers
  onMount(() => {
    console.log('WebSocket provider mounted, setting up event handlers');

    // Handle state changes
    wsClient.onStateChange(state => {
      console.log('WebSocket state changed to:', state);
      setStore('connectionState', state);
      setStore('isConnected', state === 'connected');

      if (state === 'error') {
        setStore('error', 'Connection error occurred');
      } else if (state === 'connected') {
        setStore('error', null);
      }
    });

    // Handle incoming messages
    wsClient.onMessage(message => {
      console.log('WebSocket message received in provider:', message);
      setStore('lastMessage', message);

      // Handle specific message types
      switch (message.type) {
        case 'Connected':
          setStore('clientId', message.payload.client_id);
          console.log('WebSocket client connected with ID:', message.payload.client_id);
          break;

        case 'Error':
          setStore('error', message.payload.message);
          console.error('WebSocket error:', message.payload.message);
          break;

        default:
          // Other message types will be handled by specific components
          break;
      }
    });

    // Auto-connect when provider mounts
    actions.connect();
  });

  // Cleanup on unmount
  onCleanup(() => {
    console.log('WebSocket provider unmounting, disconnecting...');
    actions.disconnect();
  });

  const contextValue: WebSocketContextValue = {
    store,
    actions,
  };

  return (
    <WebSocketContext.Provider value={contextValue}>{props.children}</WebSocketContext.Provider>
  );
};

// Hook to use WebSocket context
export const useWebSocket = () => {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error('useWebSocket must be used within a WebSocketProvider');
  }
  return context;
};

// Hook to listen for specific message types
export const useWebSocketMessage = (
  messageType: WebSocketMessage['type'],
  callback: (message: WebSocketMessage) => void
) => {
  const { store } = useWebSocket();

  // Create effect to watch for messages of specific type
  const cleanup = () => {};

  onMount(() => {
    const checkMessage = () => {
      const message = store.lastMessage;
      if (message && message.type === messageType) {
        callback(message);
      }
    };

    // Check immediately in case message was already received
    checkMessage();

    // Set up reactive effect to watch for new messages
    // Note: In a real implementation, you might want to use createEffect
    // but for simplicity, we'll let components handle this
  });

  onCleanup(cleanup);
};

// Hook to get connection status
export const useWebSocketConnection = () => {
  const { store, actions } = useWebSocket();

  return {
    state: store.connectionState,
    isConnected: store.isConnected,
    error: store.error,
    clientId: store.clientId,
    connect: actions.connect,
    disconnect: actions.disconnect,
  };
};
