// Re-export all generated types from ts-rs
export * from './generated';

// Additional types not generated from Rust
export type AiSessionStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface ApiError {
  error: string;
  message?: string;
}

// WebSocket message types (not generated from Rust - client-specific)
export type WebSocketMessage =
  | { type: "Connected"; payload: { client_id: string } }
  | { type: "Disconnected"; payload: { client_id: string } }
  | { type: "ProjectCreated"; payload: { project: Project } }
  | { type: "ProjectUpdated"; payload: { project: Project } }
  | { type: "ProjectDeleted"; payload: { project_id: string } }
  | { type: "ProjectStatusChanged"; payload: { project_id: string; status: string } }
  | { type: "Error"; payload: { message: string } }
  | { type: "Ping" }
  | { type: "Pong" };

// WebSocket connection states
export type WebSocketConnectionState = 'connecting' | 'connected' | 'disconnected' | 'error';

// WebSocket client interface
export interface WebSocketClient {
  connect(): void;
  disconnect(): void;
  send(message: WebSocketMessage): void;
  onMessage(callback: (message: WebSocketMessage) => void): void;
  onStateChange(callback: (state: WebSocketConnectionState) => void): void;
  getState(): WebSocketConnectionState;
}
