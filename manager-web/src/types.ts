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

// File operation types
export interface FileInfo {
  name: string;
  path: string;
  is_directory: boolean;
  size?: number;
  modified_at?: number;
  created_at?: number;
}

export interface FileListRequest {
  project_id?: string;
  path?: string; // Relative path within project, defaults to root
}

export interface FileListResponse {
  files: FileInfo[];
  current_path: string;
}

export interface FileCreateRequest {
  project_id: string;
  path: string; // Relative path within project
  content?: string; // None for directories
  is_directory: boolean;
}

export interface FileUpdateRequest {
  project_id: string;
  content: string;
}

export interface FileContentResponse {
  path: string;
  content: string;
  modified_at?: number;
}

export interface FileResponse {
  file: FileInfo;
}
