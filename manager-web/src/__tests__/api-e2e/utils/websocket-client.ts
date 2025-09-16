import WebSocket from 'ws';

/**
 * WebSocket client for API-only e2e tests
 * Handles WebSocket connections and message testing
 */
export class TestWebSocketClient {
  private ws: WebSocket | null = null;
  private url: string;
  private messageHandlers: Map<string, (data: any) => void> = new Map();
  private eventHandlers: Map<string, (() => void)[]> = new Map();
  private receivedMessages: any[] = [];
  private isConnected = false;

  constructor(baseURL: string = 'ws://localhost:8081') {
    this.url = baseURL;
  }

  /**
   * Connect to WebSocket server
   */
  async connect(path: string = '/ws/work'): Promise<void> {
    return new Promise((resolve, reject) => {
      const fullUrl = `${this.url}${path}`;

      this.ws = new WebSocket(fullUrl);

      this.ws.onopen = () => {
        this.isConnected = true;
        this.triggerEvent('open');
        resolve();
      };

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data.toString());
          this.receivedMessages.push(data);
          this.handleMessage(data);
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };

      this.ws.onclose = () => {
        this.isConnected = false;
        this.triggerEvent('close');
      };

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        this.triggerEvent('error');
        reject(error);
      };

      // Timeout after 10 seconds
      setTimeout(() => {
        if (!this.isConnected) {
          reject(new Error('WebSocket connection timeout'));
        }
      }, 10000);
    });
  }

  /**
   * Disconnect from WebSocket server
   */
  async disconnect(): Promise<void> {
    if (this.ws && this.isConnected) {
      return new Promise((resolve) => {
        this.ws!.onclose = () => {
          this.isConnected = false;
          this.triggerEvent('close');
          resolve();
        };
        this.ws!.close();
      });
    }
  }

  /**
   * Send a message through WebSocket
   */
  send(message: any): void {
    if (this.ws && this.isConnected) {
      const messageString = JSON.stringify(message);
      this.ws.send(messageString);
    } else {
      throw new Error('WebSocket is not connected');
    }
  }

  /**
   * Register a message handler for specific message types
   */
  onMessage(type: string, handler: (data: any) => void): void {
    this.messageHandlers.set(type, handler);
  }

  /**
   * Register an event handler
   */
  onEvent(event: string, handler: () => void): void {
    if (!this.eventHandlers.has(event)) {
      this.eventHandlers.set(event, []);
    }
    this.eventHandlers.get(event)!.push(handler);
  }

  /**
   * Wait for a specific message type with timeout
   */
  async waitForMessage(type: string, timeoutMs: number = 5000): Promise<any> {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error(`Timeout waiting for message type: ${type}`));
      }, timeoutMs);

      // Check if message already received
      const existingMessage = this.receivedMessages.find(msg => msg.type === type);
      if (existingMessage) {
        clearTimeout(timeout);
        resolve(existingMessage);
        return;
      }

      // Wait for new message
      const handler = (data: any) => {
        if (data.type === type) {
          clearTimeout(timeout);
          this.messageHandlers.delete(type);
          resolve(data);
        }
      };

      this.onMessage(type, handler);
    });
  }

  /**
   * Wait for connection to be established
   */
  async waitForConnection(timeoutMs: number = 5000): Promise<void> {
    if (this.isConnected) {
      return Promise.resolve();
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Timeout waiting for WebSocket connection'));
      }, timeoutMs);

      this.onEvent('open', () => {
        clearTimeout(timeout);
        resolve();
      });
    });
  }

  /**
   * Get all received messages
   */
  getReceivedMessages(): any[] {
    return [...this.receivedMessages];
  }

  /**
   * Get messages of specific type
   */
  getMessagesByType(type: string): any[] {
    return this.receivedMessages.filter(msg => msg.type === type);
  }

  /**
   * Clear received messages
   */
  clearMessages(): void {
    this.receivedMessages = [];
  }

  /**
   * Check if client is connected
   */
  isConnected(): boolean {
    return this.isConnected;
  }

  /**
   * Get connection URL
   */
  getUrl(): string {
    return this.url;
  }

  /**
   * Handle incoming message by type
   */
  private handleMessage(data: any): void {
    const handler = this.messageHandlers.get(data.type);
    if (handler) {
      handler(data);
    }
  }

  /**
   * Trigger event handlers
   */
  private triggerEvent(event: string): void {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      handlers.forEach(handler => handler());
    }
  }
}

/**
 * WebSocket client manager for test scenarios
 */
export class WebSocketTestManager {
  private clients: Map<string, TestWebSocketClient> = new Map();

  /**
   * Create and connect a new WebSocket client
   */
  async createClient(id: string, path: string = '/ws/work'): Promise<TestWebSocketClient> {
    const client = new TestWebSocketClient();
    await client.connect(path);
    this.clients.set(id, client);
    return client;
  }

  /**
   * Get a client by ID
   */
  getClient(id: string): TestWebSocketClient | undefined {
    return this.clients.get(id);
  }

  /**
   * Disconnect and remove a client
   */
  async removeClient(id: string): Promise<void> {
    const client = this.clients.get(id);
    if (client) {
      await client.disconnect();
      this.clients.delete(id);
    }
  }

  /**
   * Disconnect all clients
   */
  async disconnectAll(): Promise<void> {
    const disconnectPromises = Array.from(this.clients.entries()).map(
      async ([id, client]) => {
        await client.disconnect();
        this.clients.delete(id);
      }
    );

    await Promise.all(disconnectPromises);
  }

  /**
   * Get all active client IDs
   */
  getActiveClients(): string[] {
    return Array.from(this.clients.keys());
  }

  /**
   * Broadcast a message to all connected clients
   */
  broadcast(message: any): void {
    this.clients.forEach(client => {
      if (client.isConnected()) {
        client.send(message);
      }
    });
  }

  /**
   * Wait for all clients to receive a specific message type
   */
  async waitForAllClientsMessage(type: string, timeoutMs: number = 5000): Promise<any[]> {
    const waitPromises = Array.from(this.clients.values()).map(client =>
      client.waitForMessage(type, timeoutMs)
    );

    return Promise.all(waitPromises);
  }
}

// Global WebSocket test manager instance
export const wsTestManager = new WebSocketTestManager();