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

export interface ApiError {
  error: string;
  message?: string;
}
