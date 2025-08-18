// Basic types for the Manager API
export interface Project {
  id: string;
  name: string;
  path: string;
  language?: string;
  framework?: string;
  status: string;
  created_at: number;
  updated_at: number;
}

export interface CreateProjectRequest {
  name: string;
  language?: string;
  framework?: string;
}

export interface AddExistingProjectRequest {
  name: string;
  path: string; // Required - must be existing directory
  language?: string;
  framework?: string;
}

export interface ApiError {
  error: string;
  message?: string;
}

// WebSocket message types (generated from Rust with ts-rs)
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
