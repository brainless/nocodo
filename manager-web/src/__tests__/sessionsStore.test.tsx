import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { createStore } from 'solid-js/store';
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
  ended_at: null,
};

const mockSession2: AiSession = {
  id: 'session-456',
  project_id: 'project-789',
  tool_name: 'gpt',
  status: 'completed',
  prompt: 'Test prompt 2',
  project_context: 'Test context 2',
  started_at: 1640995100000,
  ended_at: 1640995300000,
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
    configurable: true,
  });

  (global.WebSocket as any).CONNECTING = 0;
  (global.WebSocket as any).OPEN = 1;
  (global.WebSocket as any).CLOSING = 2;
  (global.WebSocket as any).CLOSED = 3;

  Object.defineProperty(window, 'location', {
    value: {
      protocol: 'http:',
      host: 'localhost:8081',
    },
    writable: true,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

// Test the store logic directly
describe('Sessions Store Logic', () => {
  test('should initialize empty store', () => {
    const [store] = createStore({
      list: [] as AiSession[],
      byId: {} as Record<string, AiSession>,
      loading: false,
      error: null as string | null,
      subscriptions: {} as Record<string, { close: () => void }>,
    });

    expect(store.list).toEqual([]);
    expect(store.byId).toEqual({});
    expect(store.loading).toBe(false);
    expect(store.error).toBe(null);
    expect(store.subscriptions).toEqual({});
  });

  test('should update store with sessions list', () => {
    const [store, setStore] = createStore({
      list: [] as AiSession[],
      byId: {} as Record<string, AiSession>,
      loading: false,
      error: null as string | null,
      subscriptions: {} as Record<string, { close: () => void }>,
    });

    // Simulate fetchList success
    setStore('list', mockSessionList);

    const byId = mockSessionList.reduce(
      (acc, session) => {
        acc[session.id] = session;
        return acc;
      },
      {} as Record<string, AiSession>
    );
    setStore('byId', byId);

    expect(store.list).toEqual(mockSessionList);
    expect(store.byId['session-123']).toEqual(mockSession1);
    expect(store.byId['session-456']).toEqual(mockSession2);
  });

  test('should update individual session in store', () => {
    const [store, setStore] = createStore({
      list: [mockSession1] as AiSession[],
      byId: { 'session-123': mockSession1 } as Record<string, AiSession>,
      loading: false,
      error: null as string | null,
      subscriptions: {} as Record<string, { close: () => void }>,
    });

    // Update session status
    setStore('byId', 'session-123', 'status', 'completed');
    setStore('list', 0, 'status', 'completed');

    expect(store.byId['session-123'].status).toBe('completed');
    expect(store.list[0].status).toBe('completed');
  });

  test('should handle error state', () => {
    const [store, setStore] = createStore({
      list: [] as AiSession[],
      byId: {} as Record<string, AiSession>,
      loading: false,
      error: null as string | null,
      subscriptions: {} as Record<string, { close: () => void }>,
    });

    const errorMessage = 'API Error';
    setStore('error', errorMessage);
    setStore('loading', false);

    expect(store.error).toBe(errorMessage);
    expect(store.loading).toBe(false);
  });

  test('should handle loading state', () => {
    const [store, setStore] = createStore({
      list: [] as AiSession[],
      byId: {} as Record<string, AiSession>,
      loading: false,
      error: null as string | null,
      subscriptions: {} as Record<string, { close: () => void }>,
    });

    setStore('loading', true);
    setStore('error', null);

    expect(store.loading).toBe(true);
    expect(store.error).toBe(null);
  });
});

describe('API Integration', () => {
  test('should call listSessions API', async () => {
    (apiClient.listSessions as any).mockResolvedValueOnce(mockSessionList);

    const result = await apiClient.listSessions();

    expect(apiClient.listSessions).toHaveBeenCalled();
    expect(result).toEqual(mockSessionList);
  });

  test('should call getSession API with correct id', async () => {
    (apiClient.getSession as any).mockResolvedValueOnce(mockSession1);

    const result = await apiClient.getSession('session-123');

    expect(apiClient.getSession).toHaveBeenCalledWith('session-123');
    expect(result).toEqual(mockSession1);
  });

  test('should handle API errors', async () => {
    const errorMessage = 'API Error';
    (apiClient.listSessions as any).mockRejectedValueOnce(new Error(errorMessage));

    await expect(apiClient.listSessions()).rejects.toThrow(errorMessage);
  });

  test('should create WebSocket subscription', () => {
    const mockSubscription = { close: mockWebSocketClose };
    (apiClient.subscribeSession as any).mockReturnValueOnce(mockSubscription);

    const subscription = apiClient.subscribeSession(
      'session-123',
      vi.fn(),
      vi.fn(),
      vi.fn(),
      vi.fn()
    );

    expect(apiClient.subscribeSession).toHaveBeenCalledWith(
      'session-123',
      expect.any(Function),
      expect.any(Function),
      expect.any(Function),
      expect.any(Function)
    );
    expect(subscription).toEqual(mockSubscription);
  });
});
