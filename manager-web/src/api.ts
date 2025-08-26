import {
  AiSession,
  AiSessionListResponse,
  AiSessionOutputListResponse,
  AiSessionResponse,
  ApiError,
  CreateAiSessionRequest,
  CreateProjectRequest,
  FileContentResponse,
  FileCreateRequest,
  FileListRequest,
  FileListResponse,
  FileResponse,
  FileUpdateRequest,
  Project,
} from './types';

class ApiClient {
  private baseURL = '/api';

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
      const error: ApiError = await response.json().catch(() => ({
        error: `HTTP ${response.status}`,
        message: response.statusText,
      }));
      throw new Error(error.message || error.error);
    }

    return response.json();
  }

  async fetchProjects(): Promise<Project[]> {
    const response = await this.request<{ projects: Project[] }>('/projects');
    return response.projects;
  }

  async fetchProject(id: string): Promise<Project> {
    const response = await this.request<{ project: Project }>(`/projects/${id}`);
    return response.project;
  }

  async fetchProjectDetails(id: string): Promise<{ project: Project; components: any[] }> {
    return this.request(`/projects/${id}/details`);
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
    const endpoint = `/files${queryString ? `?${queryString}` : ''}`;
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

    return this.request<FileContentResponse>(
      `/files/${encodeURIComponent(filePath)}?${queryParams.toString()}`
    );
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

  // AI session endpoints
  async createAiSession(data: CreateAiSessionRequest): Promise<AiSessionResponse> {
    return this.request('/ai/sessions', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async listAiSessions(): Promise<AiSessionListResponse> {
    return this.request('/ai/sessions');
  }

  async getAiSession(id: string): Promise<AiSessionResponse> {
    return this.request(`/ai/sessions/${id}`);
  }

  async recordAiOutput(id: string, content: string): Promise<{ ok: boolean }> {
    return this.request(`/ai/sessions/${id}/outputs`, {
      method: 'POST',
      body: JSON.stringify({ content }),
    });
  }

  async listAiOutputs(id: string): Promise<AiSessionOutputListResponse> {
    return this.request(`/ai/sessions/${id}/outputs`);
  }

  // Issue #59: Send input to a running AI session (stdin)
  async sendAiInput(id: string, content: string): Promise<{ ok: boolean }> {
    return this.request(`/ai/sessions/${id}/input`, {
      method: 'POST',
      body: JSON.stringify({ content }),
    });
  }

  // New methods for issue #32
  /**
   * List all AI sessions
   * @returns Promise resolving to an array of AiSession objects
   */
  async listSessions(): Promise<AiSession[]> {
    const response = await this.listAiSessions();
    return response.sessions;
  }

  /**
   * Get a specific AI session by ID
   * @param id - The session ID
   * @returns Promise resolving to an AiSession object
   */
  async getSession(id: string): Promise<AiSession> {
    const response = await this.getAiSession(id);
    return response.session;
  }

  /**
   * Subscribe to live updates for an AI session via WebSocket
   * @param id - The session ID
   * @param onMessage - Callback function to handle incoming messages
   * @param onError - Callback function to handle errors
   * @param onOpen - Callback function when connection opens
   * @param onClose - Callback function when connection closes
   * @returns WebSocket connection object with close method
   */
  subscribeSession(
    id: string,
    onMessage: (data: any) => void,
    onError?: (error: Error) => void,
    onOpen?: () => void,
    onClose?: () => void
  ): { close: () => void } {
    // Create WebSocket URL for AI session updates
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    const wsUrl = `${protocol}//${host}/ws/ai-sessions/${id}`;

    // Create WebSocket connection
    const ws = new WebSocket(wsUrl);

    // Set up event handlers
    ws.onopen = () => {
      console.log(`WebSocket connected for AI session ${id}`);
      if (onOpen) onOpen();
    };

    ws.onmessage = event => {
      try {
        const data = JSON.parse(event.data);
        onMessage(data);
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error);
        if (onError) onError(new Error('Failed to parse WebSocket message'));
      }
    };

    ws.onerror = event => {
      console.error(`WebSocket error for AI session ${id}:`, event);
      if (onError) onError(new Error('WebSocket connection error'));
    };

    ws.onclose = () => {
      console.log(`WebSocket closed for AI session ${id}`);
      if (onClose) onClose();
    };

    // Return close method to allow cleanup
    return {
      close: () => {
        if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
          ws.close();
        }
      },
    };
  }
}

export const apiClient = new ApiClient();
