import {
  CreateProjectRequest,
  CreateWorkRequest,
  AddMessageRequest,
  CreateAiSessionRequest,
  FileCreateRequest,
  FileUpdateRequest,
} from '../../types';

/**
 * Mock data generators for API-only e2e tests
 * Provides consistent test fixtures and data generation utilities
 */

export class TestDataGenerator {
  private idCounter = 0;

  /**
   * Generate a unique ID for test entities
   */
  private generateId(prefix = 'test'): string {
    this.idCounter++;
    return `${prefix}-${this.idCounter}-${Date.now()}`;
  }

  /**
   * Generate mock project creation data
   */
  generateProjectData(overrides: Partial<CreateProjectRequest> = {}): CreateProjectRequest {
    const defaultData: CreateProjectRequest = {
      name: `Test Project ${this.generateId('project')}`,
      language: 'rust',
      description: 'Test project for API e2e testing',
    };

    return { ...defaultData, ...overrides };
  }

  /**
   * Generate mock work creation data
   */
  generateWorkData(overrides: Partial<CreateWorkRequest> = {}): CreateWorkRequest {
    const defaultData: CreateWorkRequest = {
      title: `Test Work ${this.generateId('work')}`,
      tool_name: 'llm-agent',
      project_id: null,
    };

    return { ...defaultData, ...overrides };
  }

  /**
   * Generate mock message data
   */
  generateMessageData(overrides: Partial<AddMessageRequest> = {}): AddMessageRequest {
    const defaultData: AddMessageRequest = {
      content: 'Test message for API e2e testing',
      content_type: 'text',
      author_type: 'user',
    };

    return { ...defaultData, ...overrides };
  }

  /**
   * Generate mock AI session data
   */
  generateAiSessionData(overrides: Partial<CreateAiSessionRequest> = {}): CreateAiSessionRequest {
    const defaultData: CreateAiSessionRequest = {
      tool_name: 'llm-agent',
      project_context: null,
    };

    return { ...defaultData, ...overrides };
  }

  /**
   * Generate mock file creation data
   */
  generateFileData(overrides: Partial<FileCreateRequest> = {}): FileCreateRequest {
    const defaultData: FileCreateRequest = {
      project_id: this.generateId('project'),
      path: `test-file-${this.generateId('file')}.txt`,
      content: 'This is test file content for API e2e testing',
      encoding: 'utf-8',
    };

    return { ...defaultData, ...overrides };
  }

  /**
   * Generate mock file update data
   */
  generateFileUpdateData(overrides: Partial<FileUpdateRequest> = {}): FileUpdateRequest {
    const defaultData: FileUpdateRequest = {
      content: 'Updated test file content',
      encoding: 'utf-8',
    };

    return { ...defaultData, ...overrides };
  }

  /**
   * Generate a batch of test projects
   */
  generateProjectBatch(count: number): CreateProjectRequest[] {
    return Array.from({ length: count }, () => this.generateProjectData());
  }

  /**
   * Generate a complete test scenario with related entities
   */
  generateTestScenario(): {
    project: CreateProjectRequest;
    work: CreateWorkRequest;
    message: AddMessageRequest;
    aiSession: CreateAiSessionRequest;
    files: FileCreateRequest[];
  } {
    const project = this.generateProjectData();
    const work = this.generateWorkData({ project_id: 'project-1' }); // Will be set after creation
    const message = this.generateMessageData();
    const aiSession = this.generateAiSessionData();
    const files = [
      this.generateFileData({ project_id: 'project-1', path: 'README.md' }),
      this.generateFileData({ project_id: 'project-1', path: 'src/main.rs' }),
    ];

    return {
      project,
      work,
      message,
      aiSession,
      files,
    };
  }

  /**
   * Generate LLM agent test prompts
   */
  generateLlmPrompts(): { fileListing: string; fileReading: string; codeAnalysis: string } {
    return {
      fileListing: 'List all files in the root directory',
      fileReading: 'Read the contents of README.md',
      codeAnalysis: 'Analyze the main.rs file and explain what it does',
    };
  }

  /**
   * Generate error test cases
   */
  generateErrorScenarios(): {
    invalidProject: CreateProjectRequest;
    invalidWork: CreateWorkRequest;
    invalidFile: FileCreateRequest;
  } {
    return {
      invalidProject: {
        name: '', // Invalid: empty name
        language: 'invalid-language',
      } as CreateProjectRequest,
      invalidWork: {
        title: '', // Invalid: empty title
        tool_name: 'invalid-tool',
      } as CreateWorkRequest,
      invalidFile: {
        project_id: 'non-existent-project',
        path: '', // Invalid: empty path
        content: 'test',
      } as FileCreateRequest,
    };
  }

  /**
   * Reset ID counter for consistent test runs
   */
  reset(): void {
    this.idCounter = 0;
  }
}

// Global test data generator instance
export const testDataGenerator = new TestDataGenerator();