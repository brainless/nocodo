// Re-export all generated types from ts-rs
export * from './types/generated/AiSession';
export * from './types/generated/Project';
export * from './types/generated/AiSessionListResponse';
export * from './types/generated/AiSessionResponse';
export * from './types/generated/CreateAiSessionRequest';
export * from './types/generated/CreateProjectRequest';
export * from './types/generated/FileInfo';
export * from './types/generated/FileListRequest';
export * from './types/generated/FileListResponse';
export * from './types/generated/FileCreateRequest';
export * from './types/generated/FileUpdateRequest';
export * from './types/generated/FileContentResponse';
export * from './types/generated/FileResponse';
export * from './types/generated/AiSessionOutput';
export * from './types/generated/AiSessionOutputListResponse';
export * from './types/generated/ProjectListResponse';
export * from './types/generated/ProjectResponse';
export * from './types/generated/AddExistingProjectRequest';
export * from './types/generated/RecordAiOutputRequest';
export * from './types/generated/ServerStatus';

// Additional types not generated from Rust
export type AiSessionStatus =
  | 'pending'
  | 'started'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface ApiError {
  error: string;
  message?: string;
}

// Forward declare Project interface for WebSocket types
import type { Project } from './types/generated/Project';

// WebSocket message types (not generated from Rust - client-specific)
export type WebSocketMessage =
  | { type: 'Connected'; payload: { client_id: string } }
  | { type: 'Disconnected'; payload: { client_id: string } }
  | { type: 'ProjectCreated'; payload: { project: Project } }
  | { type: 'ProjectUpdated'; payload: { project: Project } }
  | { type: 'ProjectDeleted'; payload: { project_id: string } }
  | { type: 'ProjectStatusChanged'; payload: { project_id: string; status: string } }
  | { type: 'Error'; payload: { message: string } }
  | { type: 'Ping' }
  | { type: 'Pong' };

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
