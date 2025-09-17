import { testApiClient } from '../setup/api-client';
import type { ExtendedAiSession } from '../../types.ts';
import type { Project, WorkResponse } from '../../types/generated';

/**
 * State manager utilities for testing Solid state management integration
 * Simulates store behavior without UI components
 */
export class TestStateManager {
  private projects: Map<string, Project> = new Map();
  private workSessions: Map<string, WorkResponse> = new Map();
  private aiSessions: Map<string, ExtendedAiSession> = new Map();
  private listeners: Map<string, ((data: any) => void)[]> = new Map();

  /**
   * Initialize state manager with API data
   */
  async initialize(): Promise<void> {
    // Load initial data from API
    const projects = await testApiClient.fetchProjects();
    projects.forEach(project => this.projects.set(project.id, project));

    const workList = await testApiClient.listWork();
    for (const work of workList.works) {
      this.workSessions.set(work.id, { work });
    }
  }

  /**
   * Project state management
   */
  getProjects(): Project[] {
    return Array.from(this.projects.values());
  }

  getProject(id: string): Project | undefined {
    return this.projects.get(id);
  }

  async addProject(
    projectData: Parameters<typeof testApiClient.createProject>[0]
  ): Promise<Project> {
    const project = await testApiClient.createProject(projectData);
    this.projects.set(project.id, project);
    this.notifyListeners('project-added', project);
    return project;
  }

  async updateProject(id: string, updates: Partial<Project>): Promise<Project> {
    // Note: Current API doesn't support project updates
    // This simulates the expected behavior
    const existing = this.projects.get(id);
    if (!existing) {
      throw new Error(`Project ${id} not found`);
    }

    const updated = { ...existing, ...updates };
    this.projects.set(id, updated);
    this.notifyListeners('project-updated', updated);
    return updated;
  }

  async removeProject(id: string): Promise<void> {
    await testApiClient.deleteProject(id);
    this.projects.delete(id);
    this.notifyListeners('project-removed', { id });
  }

  /**
   * Work session state management
   */
  getWorkSessions(): WorkResponse[] {
    return Array.from(this.workSessions.values());
  }

  getWorkSession(id: string): WorkResponse | undefined {
    return this.workSessions.get(id);
  }

  async addWorkSession(
    workData: Parameters<typeof testApiClient.createWork>[0]
  ): Promise<WorkResponse> {
    const work = await testApiClient.createWork(workData);
    this.workSessions.set(work.work.id, work);
    this.notifyListeners('work-added', work);
    return work;
  }

  async updateWorkSession(id: string): Promise<WorkResponse> {
    const work = await testApiClient.getWork(id);
    this.workSessions.set(id, work);
    this.notifyListeners('work-updated', work);
    return work;
  }

  /**
   * AI Session state management
   */
  getAiSessions(): ExtendedAiSession[] {
    return Array.from(this.aiSessions.values());
  }

  getAiSession(id: string): ExtendedAiSession | undefined {
    return this.aiSessions.get(id);
  }

  async addAiSession(
    workId: string,
    sessionData: Parameters<typeof testApiClient.createAiSession>[1]
  ): Promise<ExtendedAiSession> {
    const session = await testApiClient.createAiSession(workId, sessionData);

    // Transform to ExtendedAiSession format
    const extendedSession: ExtendedAiSession = {
      id: session.session.id,
      work_id: workId,
      message_id: '', // Would be populated from work session
      tool_name: session.session.tool_name,
      status: session.session.status,
      project_context: session.session.project_context,
      started_at: session.session.started_at,
      ended_at: session.session.ended_at,
    };

    this.aiSessions.set(session.session.id, extendedSession);
    this.notifyListeners('ai-session-added', extendedSession);
    return extendedSession;
  }

  async updateAiSession(id: string): Promise<ExtendedAiSession> {
    // Get work session to find the AI session details
    const workSession = await testApiClient.getWork(id);
    // Note: This is simplified - in real implementation would track AI session separately

    const existing = this.aiSessions.get(id);
    if (!existing) {
      throw new Error(`AI session ${id} not found`);
    }

    // Simulate status update
    const updated = { ...existing, status: 'completed' };
    this.aiSessions.set(id, updated);
    this.notifyListeners('ai-session-updated', updated);
    return updated;
  }

  /**
   * Listener management (simulates Solid store subscriptions)
   */
  subscribe(event: string, listener: (data: any) => void): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, []);
    }
    this.listeners.get(event)!.push(listener);

    // Return unsubscribe function
    return () => {
      const eventListeners = this.listeners.get(event);
      if (eventListeners) {
        const index = eventListeners.indexOf(listener);
        if (index > -1) {
          eventListeners.splice(index, 1);
        }
      }
    };
  }

  private notifyListeners(event: string, data: any): void {
    const eventListeners = this.listeners.get(event);
    if (eventListeners) {
      eventListeners.forEach(listener => listener(data));
    }
  }

  /**
   * State validation utilities
   */
  validateStateConsistency(): { valid: boolean; errors: string[] } {
    const errors: string[] = [];

    // Check that all work sessions have valid projects if project_id is set
    for (const work of this.workSessions.values()) {
      if (work.work.project_id && !this.projects.has(work.work.project_id)) {
        errors.push(
          `Work session ${work.work.id} references non-existent project ${work.work.project_id}`
        );
      }
    }

    // Check that all AI sessions reference valid work sessions
    for (const session of this.aiSessions.values()) {
      if (!this.workSessions.has(session.work_id)) {
        errors.push(
          `AI session ${session.id} references non-existent work session ${session.work_id}`
        );
      }
    }

    return {
      valid: errors.length === 0,
      errors,
    };
  }

  /**
   * State synchronization with API
   */
  async syncWithAPI(): Promise<void> {
    // Re-fetch all data from API to ensure consistency
    const projects = await testApiClient.fetchProjects();
    this.projects.clear();
    projects.forEach(project => this.projects.set(project.id, project));

    const workList = await testApiClient.listWork();
    this.workSessions.clear();
    for (const work of workList.works) {
      this.workSessions.set(work.id, { work });
    }

    this.notifyListeners('state-synced', {
      projectsCount: projects.length,
      workSessionsCount: workList.works.length,
    });
  }

  /**
   * Cleanup utilities
   */
  clearState(): void {
    this.projects.clear();
    this.workSessions.clear();
    this.aiSessions.clear();
    this.listeners.clear();
  }

  getStateSummary(): {
    projects: number;
    workSessions: number;
    aiSessions: number;
    listeners: number;
  } {
    return {
      projects: this.projects.size,
      workSessions: this.workSessions.size,
      aiSessions: this.aiSessions.size,
      listeners: this.listeners.size,
    };
  }
}

// Global test state manager instance
export const testStateManager = new TestStateManager();
