import { WebSocketMessage, WebSocketConnectionState, WebSocketClient } from './types';

/**
 * WebSocket client for real-time communication with the Manager daemon
 */
export class RealtimeWebSocketClient implements WebSocketClient {
  private socket: WebSocket | null = null;
  private url: string;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000; // Start with 1 second
  private pingInterval: number | null = null;
  private connectionState: WebSocketConnectionState = 'disconnected';

  private messageCallbacks: ((message: WebSocketMessage) => void)[] = [];
  private stateCallbacks: ((state: WebSocketConnectionState) => void)[] = [];

  constructor(url: string = '/ws') {
    // Handle both absolute and relative URLs
    if (url.startsWith('/')) {
      // Convert relative URL to WebSocket URL
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = window.location.host;
      this.url = `${protocol}//${host}${url}`;
    } else {
      this.url = url;
    }

    console.log('WebSocket client initialized with URL:', this.url);
  }

  connect(): void {
    if (this.socket?.readyState === WebSocket.OPEN) {
      console.log('WebSocket already connected');
      return;
    }

    console.log('Attempting to connect to WebSocket...');
    this.setState('connecting');

    try {
      this.socket = new WebSocket(this.url);
      this.setupEventHandlers();
    } catch (error) {
      console.error('Failed to create WebSocket connection:', error);
      this.setState('error');
      this.attemptReconnect();
    }
  }

  disconnect(): void {
    console.log('Manually disconnecting WebSocket');
    this.reconnectAttempts = this.maxReconnectAttempts; // Prevent reconnection

    if (this.pingInterval) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }

    if (this.socket) {
      this.socket.close(1000, 'Manual disconnect');
      this.socket = null;
    }

    this.setState('disconnected');
  }

  send(message: WebSocketMessage): void {
    if (this.socket?.readyState === WebSocket.OPEN) {
      const json = JSON.stringify(message);
      this.socket.send(json);
      console.debug('WebSocket message sent:', message);
    } else {
      console.warn('Cannot send message: WebSocket is not connected', message);
    }
  }

  onMessage(callback: (message: WebSocketMessage) => void): void {
    this.messageCallbacks.push(callback);
  }

  onStateChange(callback: (state: WebSocketConnectionState) => void): void {
    this.stateCallbacks.push(callback);
  }

  getState(): WebSocketConnectionState {
    return this.connectionState;
  }

  private setupEventHandlers(): void {
    if (!this.socket) return;

    this.socket.onopen = () => {
      console.log('WebSocket connection opened');
      this.setState('connected');
      this.reconnectAttempts = 0;
      this.reconnectDelay = 1000; // Reset delay
      this.startPingInterval();
    };

    this.socket.onclose = event => {
      console.log('WebSocket connection closed:', event.code, event.reason);
      this.setState('disconnected');

      if (this.pingInterval) {
        clearInterval(this.pingInterval);
        this.pingInterval = null;
      }

      // Attempt reconnection if it wasn't a manual disconnect
      if (event.code !== 1000 && this.reconnectAttempts < this.maxReconnectAttempts) {
        this.attemptReconnect();
      }
    };

    this.socket.onerror = error => {
      console.error('WebSocket error:', error);
      this.setState('error');
    };

    this.socket.onmessage = event => {
      try {
        const message: WebSocketMessage = JSON.parse(event.data);
        console.debug('WebSocket message received:', message);

        // Handle ping/pong internally
        if (message.type === 'Ping') {
          this.send({ type: 'Pong' });
          return;
        }

        // Notify all callbacks
        this.messageCallbacks.forEach(callback => {
          try {
            callback(message);
          } catch (error) {
            console.error('Error in message callback:', error);
          }
        });
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error);
        console.debug('Raw message:', event.data);
      }
    };
  }

  private setState(state: WebSocketConnectionState): void {
    if (this.connectionState !== state) {
      console.log(`WebSocket state changed: ${this.connectionState} → ${state}`);
      this.connectionState = state;

      // Notify all state change callbacks
      this.stateCallbacks.forEach(callback => {
        try {
          callback(state);
        } catch (error) {
          console.error('Error in state change callback:', error);
        }
      });
    }
  }

  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.log('Max reconnection attempts reached');
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1); // Exponential backoff

    console.log(
      `Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`
    );

    setTimeout(() => {
      if (this.connectionState === 'disconnected' || this.connectionState === 'error') {
        this.connect();
      }
    }, delay);
  }

  private startPingInterval(): void {
    // Send ping every 30 seconds to keep connection alive
    this.pingInterval = window.setInterval(() => {
      this.send({ type: 'Ping' });
    }, 30000);
  }
}

// Global WebSocket client instance
let wsClient: RealtimeWebSocketClient | null = null;

/**
 * Get the global WebSocket client instance
 */
export function getWebSocketClient(): RealtimeWebSocketClient {
  if (!wsClient) {
    wsClient = new RealtimeWebSocketClient();
  }
  return wsClient;
}

/**
 * Initialize and connect the WebSocket client
 */
export function initializeWebSocket(): RealtimeWebSocketClient {
  const client = getWebSocketClient();
  client.connect();
  return client;
}
