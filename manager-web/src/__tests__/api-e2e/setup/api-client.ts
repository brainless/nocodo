import {
  AddMessageRequest,
  AiSessionListResponse,
  AiSessionOutputListResponse,
  AiSessionResponse,
  CreateAiSessionRequest,
  CreateProjectRequest,
  CreateWorkRequest,
  FileContentResponse,
  FileCreateRequest,
  FileListRequest,
  FileListResponse,
  FileResponse,
  FileUpdateRequest,
  Project,
  WorkMessageResponse,
  WorkResponse,
} from '../../types';

/**
 * API client for API-only end-to-end tests
 * Makes direct HTTP calls to the test server without browser dependencies
 */
export class TestApiClient {
  private baseURL: string;

  constructor(baseURL: string = 'http://localhost:8081') {
    this.baseURL = baseURL;
  }

  private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.baseURL}${endpoint}`;

    const response = await fetch(url, {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    });

    if (!response.ok) {
      const errorText = await response.text();
      let errorMessage: string;
      try {
        const errorJson = JSON.parse(errorText);
        errorMessage = errorJson.message || errorJson.error || `HTTP ${response.status}`;
      } catch {
        errorMessage = errorText || `HTTP ${response.status}`;
      }
      throw new Error(errorMessage);
    }

    return response.json();
  }

  // Project operations
  async fetchProjects(): Promise<Project[]> {
    const response = await this.request<{ projects: Project[] }>('/api/projects');
    return response.projects;
  }

  async fetchProject(id: string): Promise<Project> {
    const response = await this.request<{ project: Project }>(`/api/projects/${id}`);
    return response.project;
  }

  async createProject(data: CreateProjectRequest): Promise<Project> {
    return this.request('/api/projects', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async deleteProject(id: string): Promise<void> {
    return this.request(`/api/projects/${id}`, {
      method: 'DELETE',
    });
  }

  // File operations
  async listFiles(params: FileListRequest): Promise<FileListResponse> {
    const queryParams = new URLSearchParams();
    if (params.project_id) queryParams.set('project_id', params.project_id);
    if (params.path) queryParams.set('path', params.path);

    const queryString = queryParams.toString();
    const endpoint = `/api/files${queryString ? `?${queryString}` : ''}`;
    return this.request<FileListResponse>(endpoint);
  }

  async createFile(data: FileCreateRequest): Promise<FileResponse> {
    return this.request('/api/files', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async getFileContent(filePath: string, projectId: string): Promise<FileContentResponse> {
    const queryParams = new URLSearchParams();
    queryParams.set('project_id', projectId);

    return this.request<FileContentResponse>(
      `/api/files/${encodeURIComponent(filePath)}?${queryParams.toString()}`
    );
  }

  async updateFile(filePath: string, data: FileUpdateRequest): Promise<FileContentResponse> {
    return this.request(`/api/files/${encodeURIComponent(filePath)}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }

  async deleteFile(filePath: string, projectId: string): Promise<void> {
    const queryParams = new URLSearchParams();
    queryParams.set('project_id', projectId);

    return this.request(`/api/files/${encodeURIComponent(filePath)}?${queryParams.toString()}`, {
      method: 'DELETE',
    });
  }

  // Work/Message operations
  async createWork(data: CreateWorkRequest): Promise<WorkResponse> {
    return this.request('/api/work', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async addMessageToWork(workId: string, data: AddMessageRequest): Promise<WorkMessageResponse> {
    return this.request(`/api/work/${workId}/messages`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async getWork(id: string): Promise<{ work: any; messages: any[]; total_messages: number }> {
    return this.request(`/api/work/${id}`);
  }

  async listWork(): Promise<{ works: any[] }> {
    return this.request('/api/work');
  }

  // AI Session operations
  async createAiSession(workId: string, data: CreateAiSessionRequest): Promise<AiSessionResponse> {
    return this.request(`/api/work/${workId}/sessions`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async recordAiOutput(id: string, content: string): Promise<{ ok: boolean }> {
    return this.request(`/api/work/${id}/outputs`, {
      method: 'POST',
      body: JSON.stringify({ content }),
    });
  }

  async listAiOutputs(id: string): Promise<AiSessionOutputListResponse> {
    try {
      return await this.request<AiSessionOutputListResponse>(`/api/work/${id}/outputs`);
    } catch (error) {
      // If outputs endpoint doesn't exist, return empty response
      console.warn(`Outputs endpoint not available for work ${id}, returning empty outputs`);
      return { outputs: [] };
    }
  }

  async sendAiInput(id: string, content: string): Promise<{ ok: boolean }> {
    return this.request(`/api/work/${id}/input`, {
      method: 'POST',
      body: JSON.stringify({ content }),
    });
  }

  // Health check
  async healthCheck(): Promise<{ status: string }> {
    return this.request('/health');
  }
}

export const testApiClient = new TestApiClient();