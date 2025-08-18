import { 
  Project, 
  CreateProjectRequest, 
  ApiError, 
  FileListRequest,
  FileListResponse,
  FileCreateRequest,
  FileUpdateRequest,
  FileContentResponse,
  FileResponse
} from './types';

class ApiClient {
  private baseURL = '/api';

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseURL}${endpoint}`;
    
    const response = await fetch(url, {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    });
    
    if (!response.ok) {
      const error: ApiError = await response.json().catch(() => ({
        error: `HTTP ${response.status}`,
        message: response.statusText,
      }));
      throw new Error(error.message || error.error);
    }
    
    return response.json();
  }

  async fetchProjects(): Promise<Project[]> {
    const response = await this.request<{projects: Project[]}>('/projects');
    return response.projects;
  }
  
  async createProject(data: CreateProjectRequest): Promise<Project> {
    return this.request('/projects', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }
  
  async deleteProject(id: string): Promise<void> {
    return this.request(`/projects/${id}`, {
      method: 'DELETE',
    });
  }
  
  // File operations
  async listFiles(params: FileListRequest): Promise<FileListResponse> {
    const queryParams = new URLSearchParams();
    if (params.project_id) queryParams.set('project_id', params.project_id);
    if (params.path) queryParams.set('path', params.path);
    
    const queryString = queryParams.toString();
    const endpoint = `/files${queryString ? '?' + queryString : ''}`;
    return this.request<FileListResponse>(endpoint);
  }
  
  async createFile(data: FileCreateRequest): Promise<FileResponse> {
    return this.request('/files', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }
  
  async getFileContent(filePath: string, projectId: string): Promise<FileContentResponse> {
    const queryParams = new URLSearchParams();
    queryParams.set('project_id', projectId);
    
    return this.request<FileContentResponse>(`/files/${encodeURIComponent(filePath)}?${queryParams.toString()}`);
  }
  
  async updateFile(filePath: string, data: FileUpdateRequest): Promise<FileContentResponse> {
    return this.request(`/files/${encodeURIComponent(filePath)}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }
  
  async deleteFile(filePath: string, projectId: string): Promise<void> {
    const queryParams = new URLSearchParams();
    queryParams.set('project_id', projectId);
    
    return this.request(`/files/${encodeURIComponent(filePath)}?${queryParams.toString()}`, {
      method: 'DELETE',
    });
  }
}

export const apiClient = new ApiClient();
