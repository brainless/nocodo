import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, cleanup } from '@solidjs/testing-library';
import { createRoot } from 'solid-js';
import { SessionsProvider, useSessions, useSessionsList, useSession } from '../stores/sessionsStore';
import { apiClient } from '../api';
import type { AiSession } from '../types';

vi.mock('../api');

// Mock data
const mockSession1: AiSession = {
  id: 'session-123',
  project_id: 'project-456',
  tool_name: 'claude',
  status: 'running',
  prompt: 'Test prompt 1',
  project_context: 'Test context 1',
  started_at: 1640995200000,
  ended_at: undefined
};

const mockSession2: AiSession = {
  id: 'session-456',
  project_id: 'project-789',
  tool_name: 'gpt',
  status: 'completed',
  prompt: 'Test prompt 2',
  project_context: 'Test context 2',
  started_at: 1640995100000,
  ended_at: 1640995300000
};

const mockSessionList = [mockSession1, mockSession2];

// Mock WebSocket
let mockWebSocket: any;
const mockWebSocketClose = vi.fn();

beforeEach(() => {
  vi.resetAllMocks();
  
  // Setup WebSocket mock
  mockWebSocket = {
    readyState: 1, // OPEN
    close: mockWebSocketClose,
    onopen: null,
    onmessage: null,
    onerror: null,
    onclose: null,
  };
  
  Object.defineProperty(global, 'WebSocket', {
    value: vi.fn().mockImplementation(() => mockWebSocket),
    writable: true,
    configurable: true
  });
  
  (global.WebSocket as any).CONNECTING = 0;
  (global.WebSocket as any).OPEN = 1;
  (global.WebSocket as any).CLOSING = 2;
  (global.WebSocket as any).CLOSED = 3;
  
  Object.defineProperty(window, 'location', {
    value: {
      protocol: 'http:',
      host: 'localhost:8081'
    },
    writable: true
  });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('SessionsStore', () => {
  describe('SessionsProvider', () => {
    test('should provide context value', () => {
      createRoot(dispose => {
        let contextValue: any;
        
        const TestComponent = () => {
          contextValue = useSessions();
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        expect(contextValue).toBeDefined();
        expect(contextValue.store).toBeDefined();
        expect(contextValue.actions).toBeDefined();
        expect(contextValue.store.list).toEqual([]);
        expect(contextValue.store.byId).toEqual({});
        expect(contextValue.store.loading).toBe(false);
        expect(contextValue.store.error).toBe(null);
        
        dispose();
      });
    });
    
    test('should throw error when used outside provider', () => {
      createRoot(dispose => {
        expect(() => {
          const TestComponent = () => {
            useSessions();
            return null;
          };
          render(() => <TestComponent />);
        }).toThrow('useSessions must be used within a SessionsProvider');
        
        dispose();
      });
    });
  });

  describe('fetchList action', () => {
    test('should successfully fetch and store sessions list', async () => {
      (apiClient.listSessions as any).mockResolvedValueOnce(mockSessionList);
      
      createRoot(async dispose => {
        let store: any;
        let actions: any;
        
        const TestComponent = () => {
          const context = useSessions();
          store = context.store;
          actions = context.actions;
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        expect(store.loading).toBe(false);
        expect(store.list).toEqual([]);
        
        await actions.fetchList();
        
        expect(apiClient.listSessions).toHaveBeenCalled();
        expect(store.list).toEqual(mockSessionList);
        expect(store.byId['session-123']).toEqual(mockSession1);
        expect(store.byId['session-456']).toEqual(mockSession2);
        expect(store.loading).toBe(false);
        expect(store.error).toBe(null);
        
        dispose();
      });
    });
    
    test('should handle fetchList error', async () => {
      const errorMessage = 'API Error';
      (apiClient.listSessions as any).mockRejectedValueOnce(new Error(errorMessage));
      
      createRoot(async dispose => {
        let store: any;
        let actions: any;
        
        const TestComponent = () => {
          const context = useSessions();
          store = context.store;
          actions = context.actions;
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        await actions.fetchList();
        
        expect(store.list).toEqual([]);
        expect(store.error).toBe(errorMessage);
        expect(store.loading).toBe(false);
        
        dispose();
      });
    });
  });

  describe('fetchById action', () => {
    test('should successfully fetch and store session by id', async () => {
      (apiClient.getSession as any).mockResolvedValueOnce(mockSession1);
      
      createRoot(async dispose => {
        let store: any;
        let actions: any;
        
        const TestComponent = () => {
          const context = useSessions();
          store = context.store;
          actions = context.actions;
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        const result = await actions.fetchById('session-123');
        
        expect(apiClient.getSession).toHaveBeenCalledWith('session-123');
        expect(result).toEqual(mockSession1);
        expect(store.byId['session-123']).toEqual(mockSession1);
        expect(store.list).toContain(mockSession1);
        expect(store.error).toBe(null);
        
        dispose();
      });
    });
    
    test('should update existing session in list when fetching by id', async () => {
      const updatedSession = { ...mockSession1, status: 'completed' };
      (apiClient.getSession as any).mockResolvedValueOnce(updatedSession);
      
      createRoot(async dispose => {
        let store: any;
        let actions: any;
        
        const TestComponent = () => {
          const context = useSessions();
          store = context.store;
          actions = context.actions;
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        // Set initial state
        store.list = [mockSession1];
        store.byId = { 'session-123': mockSession1 };
        
        await actions.fetchById('session-123');
        
        expect(store.byId['session-123'].status).toBe('completed');
        expect(store.list[0].status).toBe('completed');
        
        dispose();
      });
    });
    
    test('should handle fetchById error', async () => {
      const errorMessage = 'Session not found';
      (apiClient.getSession as any).mockRejectedValueOnce(new Error(errorMessage));
      
      createRoot(async dispose => {
        let store: any;
        let actions: any;
        
        const TestComponent = () => {
          const context = useSessions();
          store = context.store;
          actions = context.actions;
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        const result = await actions.fetchById('nonexistent');
        
        expect(result).toBe(null);
        expect(store.error).toBe(errorMessage);
        
        dispose();
      });
    });
  });

  describe('connect and disconnect actions', () => {
    test('should successfully connect to session WebSocket', async () => {
      const mockSubscription = { close: mockWebSocketClose };
      (apiClient.subscribeSession as any).mockReturnValueOnce(mockSubscription);
      
      createRoot(async dispose => {
        let store: any;
        let actions: any;
        
        const TestComponent = () => {
          const context = useSessions();
          store = context.store;
          actions = context.actions;
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        await actions.connect('session-123');
        
        expect(apiClient.subscribeSession).toHaveBeenCalledWith(
          'session-123',
          expect.any(Function),
          expect.any(Function),
          expect.any(Function),
          expect.any(Function)
        );
        expect(store.subscriptions['session-123']).toEqual(mockSubscription);
        
        dispose();
      });
    });
    
    test('should not connect if already connected', async () => {
      const mockSubscription = { close: mockWebSocketClose };
      
      createRoot(async dispose => {
        let store: any;
        let actions: any;
        
        const TestComponent = () => {
          const context = useSessions();
          store = context.store;
          actions = context.actions;
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        // Set existing subscription
        store.subscriptions = { 'session-123': mockSubscription };
        
        await actions.connect('session-123');
        
        expect(apiClient.subscribeSession).not.toHaveBeenCalled();
        
        dispose();
      });
    });
    
    test('should disconnect from session WebSocket', () => {
      createRoot(dispose => {
        let store: any;
        let actions: any;
        
        const TestComponent = () => {
          const context = useSessions();
          store = context.store;
          actions = context.actions;
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        const mockSubscription = { close: mockWebSocketClose };
        store.subscriptions = { 'session-123': mockSubscription };
        
        actions.disconnect('session-123');
        
        expect(mockWebSocketClose).toHaveBeenCalled();
        expect(store.subscriptions['session-123']).toBeUndefined();
        
        dispose();
      });
    });
  });

  describe('updateSessionStatus action', () => {
    test('should update session status in both byId and list', () => {
      createRoot(dispose => {
        let store: any;
        let actions: any;
        
        const TestComponent = () => {
          const context = useSessions();
          store = context.store;
          actions = context.actions;
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        // Set initial state
        store.list = [mockSession1];
        store.byId = { 'session-123': mockSession1 };
        
        actions.updateSessionStatus('session-123', 'completed');
        
        expect(store.byId['session-123'].status).toBe('completed');
        expect(store.list[0].status).toBe('completed');
        
        dispose();
      });
    });
  });

  describe('useSessionsList hook', () => {
    test('should return sessions list with loading and error state', () => {
      createRoot(dispose => {
        let hookResult: any;
        
        const TestComponent = () => {
          hookResult = useSessionsList();
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        expect(hookResult.sessions).toEqual([]);
        expect(hookResult.loading).toBe(false);
        expect(hookResult.error).toBe(null);
        
        dispose();
      });
    });
  });

  describe('useSession hook', () => {
    test('should return specific session by id', () => {
      createRoot(dispose => {
        let hookResult: any;
        
        const TestComponent = () => {
          hookResult = useSession('session-123');
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        expect(hookResult.session).toBe(null);
        expect(hookResult.loading).toBe(false);
        expect(hookResult.error).toBe(null);
        
        dispose();
      });
    });
    
    test('should return session when it exists in store', () => {
      createRoot(dispose => {
        let store: any;
        let hookResult: any;
        
        const TestComponent = () => {
          const context = useSessions();
          store = context.store;
          hookResult = useSession('session-123');
          return null;
        };
        
        render(() => (
          <SessionsProvider>
            <TestComponent />
          </SessionsProvider>
        ));
        
        // Set session in store
        store.byId = { 'session-123': mockSession1 };
        
        expect(hookResult.session).toEqual(mockSession1);
        
        dispose();
      });
    });
  });
});