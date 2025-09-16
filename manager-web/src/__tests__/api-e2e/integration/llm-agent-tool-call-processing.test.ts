import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';

describe('LLM Agent Tool Call Processing - API Only End-to-End', () => {
  let testProjectId: string;
  let testWorkId: string;

  beforeAll(async () => {
    await testDatabase.setupTestDatabase();
    await testServer.startServer();

    // Create test project and work session
    const projectData = testDataGenerator.generateProjectData();
    const project = await testApiClient.createProject(projectData);
    testProjectId = project.id;

    const workData = testDataGenerator.generateWorkData({
      project_id: testProjectId,
    });
    const work = await testApiClient.createWork(workData);
    testWorkId = work.work.id;
  }, 30000);

  afterAll(async () => {
    // Clean up test work and project
    if (testWorkId) {
      // Note: May need to handle cleanup more gracefully
    }
    if (testProjectId) {
      await testApiClient.deleteProject(testProjectId);
    }

    await testServer.stopServer();
    await testDatabase.cleanupTestDatabase();
  });

  beforeEach(async () => {
    testDataGenerator.reset();
  });

  describe('LLM Agent Session Lifecycle', () => {
    it('should create and manage LLM agent session lifecycle', async () => {
      const llmSessionData = testDataGenerator.generateLlmAgentSessionData({
        provider: 'openai',
        model: 'gpt-4',
        system_prompt: 'You are a helpful AI assistant with file system tools.',
      });

      const session = await testApiClient.createLlmAgentSession(testWorkId, llmSessionData);
      expect(session.session).toBeDefined();
      expect(session.session.work_id).toBe(testWorkId);
      expect(session.session.provider).toBe('openai');
      expect(session.session.model).toBe('gpt-4');
      expect(session.session.status).toBe('running');

      const sessionId = session.session.id;

      // Get session status
      const retrievedSession = await testApiClient.getLlmAgentSession(sessionId);
      expect(retrievedSession.session.id).toBe(sessionId);
      expect(retrievedSession.session.status).toBe('running');

      // Complete session
      await testApiClient.completeLlmAgentSession(sessionId);

      // Verify session is completed
      const completedSession = await testApiClient.getLlmAgentSession(sessionId);
      expect(completedSession.session.status).toBe('completed');
    });

    it('should handle multiple LLM agent sessions for same work', async () => {
      const session1Data = testDataGenerator.generateLlmAgentSessionData({
        provider: 'openai',
        model: 'gpt-4',
      });
      const session2Data = testDataGenerator.generateLlmAgentSessionData({
        provider: 'anthropic',
        model: 'claude-3-sonnet',
      });

      const session1 = await testApiClient.createLlmAgentSession(testWorkId, session1Data);
      const session2 = await testApiClient.createLlmAgentSession(testWorkId, session2Data);

      expect(session1.session.id).not.toBe(session2.session.id);
      expect(session1.session.work_id).toBe(session2.session.work_id);
      expect(session1.session.provider).toBe('openai');
      expect(session2.session.provider).toBe('anthropic');

      // List sessions for work
      const sessions = await testApiClient.getLlmAgentSessions(testWorkId);
      expect(sessions.length).toBeGreaterThanOrEqual(2);
      const sessionIds = sessions.map(s => s.id);
      expect(sessionIds).toContain(session1.session.id);
      expect(sessionIds).toContain(session2.session.id);
    });
  });

  describe('Tool Call Processing Integration', () => {
    let sessionId: string;

    beforeAll(async () => {
      // Create LLM agent session for tool call testing
      const llmSessionData = testDataGenerator.generateLlmAgentSessionData();
      const session = await testApiClient.createLlmAgentSession(testWorkId, llmSessionData);
      sessionId = session.session.id;

      // Create test files for tool operations
      const testFiles = [
        testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'README.md',
          content: '# Test Project\n\nThis is a test project for LLM agent tool call processing.',
        }),
        testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'src/main.rs',
          content: 'fn main() {\n    println!("Hello, World!");\n}',
        }),
      ];

      await Promise.all(testFiles.map(file => testApiClient.createFile(file)));
    });

    it('should process list_dir tool call through LLM agent', async () => {
      // Note: In a real scenario, the LLM would generate tool calls based on user messages.
      // For API-only testing, we simulate the tool call processing that would happen
      // when the LLM agent receives a message and generates tool calls.

      // Since we can't make real LLM calls in tests, we test the tool execution
      // that would be triggered by LLM tool calls

      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      expect(fileList.files.length).toBeGreaterThanOrEqual(2);
      expect(fileList.files.some(f => f.name === 'README.md')).toBe(true);
      expect(fileList.files.some(f => f.name === 'src')).toBe(true);
    });

    it('should process read_file tool call through LLM agent', async () => {
      // Test file reading capability that would be triggered by LLM tool calls
      const fileContent = await testApiClient.getFileContent('README.md', testProjectId);
      expect(fileContent.content).toContain('# Test Project');
      expect(fileContent.encoding).toBe('utf-8');
    });

    it('should process create_file tool call through LLM agent', async () => {
      const newFilePath = 'llm-generated-file.md';
      const newFileContent =
        '# Generated by LLM Agent\n\nThis file was created through tool call processing.';

      const createFileData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: newFilePath,
        content: newFileContent,
      });

      const createdFile = await testApiClient.createFile(createFileData);
      expect(createdFile.path).toBe(newFilePath);

      // Verify file was created
      const readContent = await testApiClient.getFileContent(newFilePath, testProjectId);
      expect(readContent.content).toBe(newFileContent);
    });

    it('should process update_file tool call through LLM agent', async () => {
      const filePath = 'src/main.rs';
      const updatedContent =
        'fn main() {\n    println!("Hello from LLM agent tool call processing!");\n}';

      await testApiClient.updateFile(filePath, {
        content: updatedContent,
        encoding: 'utf-8',
        project_id: testProjectId,
      });

      // Verify file was updated
      const readContent = await testApiClient.getFileContent(filePath, testProjectId);
      expect(readContent.content).toBe(updatedContent);
      expect(readContent.content).toContain('LLM agent tool call processing');
    });
  });

  describe('Complex Tool Call Workflows', () => {
    let sessionId: string;

    beforeAll(async () => {
      const llmSessionData = testDataGenerator.generateLlmAgentSessionData();
      const session = await testApiClient.createLlmAgentSession(testWorkId, llmSessionData);
      sessionId = session.session.id;
    });

    it('should handle multi-step tool call workflow', async () => {
      // Step 1: Analyze project structure (list files)
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      expect(fileList.files.length).toBeGreaterThan(0);

      // Step 2: Read key files
      const readmeContent = await testApiClient.getFileContent('README.md', testProjectId);
      expect(readmeContent.content).toBeDefined();

      // Step 3: Create analysis report
      const analysisContent = `# Project Analysis Report

## Files Found
Total files: ${fileList.files.length}

## Key Findings
- README.md exists with ${readmeContent.content.length} characters
- Project structure appears valid

## Generated by LLM Agent Tool Calls
This report was created through automated tool call processing.`;

      const reportFile = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'analysis-report.md',
        content: analysisContent,
      });

      await testApiClient.createFile(reportFile);

      // Step 4: Verify the complete workflow
      const finalFileList = await testApiClient.listFiles({ project_id: testProjectId });
      expect(finalFileList.files.some(f => f.name === 'analysis-report.md')).toBe(true);

      const reportContent = await testApiClient.getFileContent('analysis-report.md', testProjectId);
      expect(reportContent.content).toContain('Generated by LLM Agent Tool Calls');
      expect(reportContent.content).toContain(`Total files: ${fileList.files.length}`);
    });

    it('should handle conditional tool execution', async () => {
      // Check if a specific file exists
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      const hasConfigFile = fileList.files.some(f => f.name === 'config.toml');

      if (!hasConfigFile) {
        // Create config file if it doesn't exist
        const configContent = `[project]
name = "test-project"
version = "1.0.0"

[llm_agent]
enabled = true
model = "gpt-4"`;

        const configFile = testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'config.toml',
          content: configContent,
        });

        await testApiClient.createFile(configFile);
      }

      // Verify config file exists
      const updatedFileList = await testApiClient.listFiles({ project_id: testProjectId });
      expect(updatedFileList.files.some(f => f.name === 'config.toml')).toBe(true);

      const configContent = await testApiClient.getFileContent('config.toml', testProjectId);
      expect(configContent.content).toContain('[llm_agent]');
    });
  });

  describe('Error Handling in Tool Calls', () => {
    let sessionId: string;

    beforeAll(async () => {
      const llmSessionData = testDataGenerator.generateLlmAgentSessionData();
      const session = await testApiClient.createLlmAgentSession(testWorkId, llmSessionData);
      sessionId = session.session.id;
    });

    it('should handle file not found errors gracefully', async () => {
      await expect(
        testApiClient.getFileContent('non-existent-file.xyz', testProjectId)
      ).rejects.toThrow();
    });

    it('should handle invalid file operations', async () => {
      // Try to create file with invalid path
      const invalidFile = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: '', // Invalid empty path
        content: 'test',
      });

      await expect(testApiClient.createFile(invalidFile)).rejects.toThrow();
    });

    it('should maintain session integrity during errors', async () => {
      // Attempt operations that will fail
      try {
        await testApiClient.getFileContent('non-existent.xyz', testProjectId);
      } catch (error) {
        // Expected error
      }

      // Session should still be accessible
      const session = await testApiClient.getLlmAgentSession(sessionId);
      expect(session.session.id).toBe(sessionId);
      expect(session.session.status).toBe('running');
    });
  });

  describe('Performance and Reliability', () => {
    let sessionId: string;

    beforeAll(async () => {
      const llmSessionData = testDataGenerator.generateLlmAgentSessionData();
      const session = await testApiClient.createLlmAgentSession(testWorkId, llmSessionData);
      sessionId = session.session.id;
    });

    it('should handle rapid sequential tool calls', async () => {
      const startTime = Date.now();

      // Create multiple files rapidly
      const fileOperations = [];
      for (let i = 0; i < 5; i++) {
        const fileData = testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: `perf-test-${i}.txt`,
          content: `Performance test file ${i}\nCreated at ${new Date().toISOString()}`,
        });
        fileOperations.push(testApiClient.createFile(fileData));
      }

      await Promise.all(fileOperations);
      const endTime = Date.now();
      const duration = endTime - startTime;

      // Verify all files were created
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      const perfFiles = fileList.files.filter(f => f.name.startsWith('perf-test-'));
      expect(perfFiles.length).toBe(5);

      // Performance should be reasonable (less than 10 seconds for 5 files)
      expect(duration).toBeLessThan(10000);
    });

    it('should maintain state consistency across operations', async () => {
      const testFile = 'state-test.txt';

      // Create file
      const createData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: testFile,
        content: 'Initial state',
      });
      await testApiClient.createFile(createData);

      // Read file
      const readContent1 = await testApiClient.getFileContent(testFile, testProjectId);
      expect(readContent1.content).toBe('Initial state');

      // Update file
      await testApiClient.updateFile(testFile, {
        content: 'Updated state',
        encoding: 'utf-8',
        project_id: testProjectId,
      });

      // Read updated file
      const readContent2 = await testApiClient.getFileContent(testFile, testProjectId);
      expect(readContent2.content).toBe('Updated state');

      // Delete file
      await testApiClient.deleteFile(testFile, testProjectId);

      // Verify deletion
      await expect(testApiClient.getFileContent(testFile, testProjectId)).rejects.toThrow();
    });
  });
});
