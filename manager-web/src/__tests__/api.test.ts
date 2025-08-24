import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { apiClient } from '../api';
import type { AiSession, AiSessionListResponse, AiSessionResponse } from '../types';

// Mock data
const mockSession: AiSession = {
  id: 'session-123',
  project_id: 'project-456',
  tool_name: 'claude',
  status: 'running',
  prompt: 'Test prompt',
  project_context: 'Test context',
  started_at: 1640995200000,
  ended_at: null,
};

const mockSessionList: AiSession[] = [
  mockSession,
  {
    id: 'session-456',
    project_id: 'project-789',
    tool_name: 'gpt',
    status: 'completed',
    prompt: 'Another prompt',
    project_context: 'Another context',
    started_at: 1640995100000,
    ended_at: 1640995300000,
  },
];

describe('API Client - AI Sessions', () => {
  beforeEach(() => {
    // Reset all mocks before each test
    vi.resetAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('listSessions', () => {
    test('should fetch and return list of AI sessions', async () => {
      const mockResponse: AiSessionListResponse = {
        sessions: mockSessionList,
      };

      // Mock successful fetch response
      (global.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await apiClient.listSessions();

      expect(global.fetch).toHaveBeenCalledWith('/api/ai/sessions', {
        headers: {
          'Content-Type': 'application/json',
        },
      });
      expect(result).toEqual(mockSessionList);
      expect(result).toHaveLength(2);
      expect(result[0].id).toBe('session-123');
    });

    test('should handle API errors', async () => {
      const mockError = {
        error: 'Server Error',
        message: 'Internal server error',
      };

      (global.fetch as any).mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        json: async () => mockError,
      });

      await expect(apiClient.listSessions()).rejects.toThrow('Internal server error');
    });

    test('should handle network errors', async () => {
      (global.fetch as any).mockRejectedValueOnce(new Error('Network error'));

      await expect(apiClient.listSessions()).rejects.toThrow('Network error');
    });
  });

  describe('getSession', () => {
    test('should fetch and return specific AI session', async () => {
      const mockResponse: AiSessionResponse = {
        session: mockSession,
      };

      (global.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await apiClient.getSession('session-123');

      expect(global.fetch).toHaveBeenCalledWith('/api/ai/sessions/session-123', {
        headers: {
          'Content-Type': 'application/json',
        },
      });
      expect(result).toEqual(mockSession);
      expect(result.id).toBe('session-123');
    });

    test('should handle 404 not found', async () => {
      const mockError = {
        error: 'Not Found',
        message: 'Session not found',
      };

      (global.fetch as any).mockResolvedValueOnce({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        json: async () => mockError,
      });

      await expect(apiClient.getSession('nonexistent')).rejects.toThrow('Session not found');
    });
  });

  describe('subscribeSession', () => {
    let mockWebSocket: any;

    beforeEach(() => {
      // Mock WebSocket constants
      Object.defineProperty(global, 'WebSocket', {
        value: vi.fn().mockImplementation(() => mockWebSocket),
        writable: true,
        configurable: true,
      });

      // Add WebSocket constants to the constructor
      (global.WebSocket as any).CONNECTING = 0;
      (global.WebSocket as any).OPEN = 1;
      (global.WebSocket as any).CLOSING = 2;
      (global.WebSocket as any).CLOSED = 3;

      mockWebSocket = {
        readyState: 0, // CONNECTING
        close: vi.fn(),
        onopen: null,
        onmessage: null,
        onerror: null,
        onclose: null,
      };

      (global.WebSocket as any).mockImplementation(() => mockWebSocket);
    });

    test('should create WebSocket connection with correct URL', () => {
      const sessionId = 'session-123';
      const onMessage = vi.fn();

      Object.defineProperty(window, 'location', {
        value: {
          protocol: 'http:',
          host: 'localhost:8081',
        },
        writable: true,
      });

      apiClient.subscribeSession(sessionId, onMessage);

      expect(global.WebSocket).toHaveBeenCalledWith(
        'ws://localhost:8081/ws/ai-sessions/session-123'
      );
    });

    test('should use wss protocol for https', () => {
      const sessionId = 'session-123';
      const onMessage = vi.fn();

      Object.defineProperty(window, 'location', {
        value: {
          protocol: 'https:',
          host: 'example.com',
        },
        writable: true,
      });

      apiClient.subscribeSession(sessionId, onMessage);

      expect(global.WebSocket).toHaveBeenCalledWith('wss://example.com/ws/ai-sessions/session-123');
    });

    test('should handle WebSocket open event', () => {
      const sessionId = 'session-123';
      const onMessage = vi.fn();
      const onOpen = vi.fn();

      apiClient.subscribeSession(sessionId, onMessage, undefined, onOpen);

      // Simulate WebSocket open event
      mockWebSocket.onopen();

      expect(onOpen).toHaveBeenCalled();
    });

    test('should handle WebSocket message event', () => {
      const sessionId = 'session-123';
      const onMessage = vi.fn();
      const testData = { type: 'update', session: mockSession };

      apiClient.subscribeSession(sessionId, onMessage);

      // Simulate WebSocket message event
      const mockEvent = {
        data: JSON.stringify(testData),
      };
      mockWebSocket.onmessage(mockEvent);

      expect(onMessage).toHaveBeenCalledWith(testData);
    });

    test('should handle malformed JSON in message', () => {
      const sessionId = 'session-123';
      const onMessage = vi.fn();
      const onError = vi.fn();

      apiClient.subscribeSession(sessionId, onMessage, onError);

      // Simulate WebSocket message with invalid JSON
      const mockEvent = {
        data: 'invalid json',
      };
      mockWebSocket.onmessage(mockEvent);

      expect(onMessage).not.toHaveBeenCalled();
      expect(onError).toHaveBeenCalledWith(new Error('Failed to parse WebSocket message'));
    });

    test('should handle WebSocket error event', () => {
      const sessionId = 'session-123';
      const onMessage = vi.fn();
      const onError = vi.fn();

      apiClient.subscribeSession(sessionId, onMessage, onError);

      // Simulate WebSocket error event
      const mockErrorEvent = new Event('error');
      mockWebSocket.onerror(mockErrorEvent);

      expect(onError).toHaveBeenCalledWith(new Error('WebSocket connection error'));
    });

    test('should handle WebSocket close event', () => {
      const sessionId = 'session-123';
      const onMessage = vi.fn();
      const onClose = vi.fn();

      apiClient.subscribeSession(sessionId, onMessage, undefined, undefined, onClose);

      // Simulate WebSocket close event
      mockWebSocket.onclose();

      expect(onClose).toHaveBeenCalled();
    });

    test('should provide close method that closes WebSocket', () => {
      const sessionId = 'session-123';
      const onMessage = vi.fn();

      // Mock WebSocket as OPEN
      mockWebSocket.readyState = (global.WebSocket as any).OPEN;

      const connection = apiClient.subscribeSession(sessionId, onMessage);

      connection.close();

      expect(mockWebSocket.close).toHaveBeenCalled();
    });

    test('should close WebSocket in CONNECTING state', () => {
      const sessionId = 'session-123';
      const onMessage = vi.fn();

      // Mock WebSocket as CONNECTING
      mockWebSocket.readyState = (global.WebSocket as any).CONNECTING;

      const connection = apiClient.subscribeSession(sessionId, onMessage);

      connection.close();

      expect(mockWebSocket.close).toHaveBeenCalled();
    });

    test('should not close already closed WebSocket', () => {
      const sessionId = 'session-123';
      const onMessage = vi.fn();

      // Mock WebSocket as CLOSED
      mockWebSocket.readyState = (global.WebSocket as any).CLOSED;

      const connection = apiClient.subscribeSession(sessionId, onMessage);

      connection.close();

      expect(mockWebSocket.close).not.toHaveBeenCalled();
    });
  });
});
