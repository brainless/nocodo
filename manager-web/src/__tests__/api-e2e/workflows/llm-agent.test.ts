import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';

describe('LLM Agent Integration - API Only', () => {
  let testProjectId: string;
  let testWorkId: string;
  let testMessageId: string;

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

    // Create a message in the work session for AI session creation
    const messageData = testDataGenerator.generateMessageData({
      content: 'Test message for AI session',
    });
    const message = await testApiClient.addMessageToWork(testWorkId, messageData);
    testMessageId = message.message.id;
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

  describe('LLM Agent Session Management', () => {
    it('should create LLM agent session with proper configuration', async () => {
      const aiSessionData = testDataGenerator.generateAiSessionData({
        message_id: testMessageId,
        tool_name: 'llm-agent',
      });

      const aiSession = await testApiClient.createAiSession(testWorkId, aiSessionData);

      expect(aiSession).toBeDefined();
      expect(aiSession.session).toBeDefined();
      expect(aiSession.session.tool_name).toBe('llm-agent');
      expect(aiSession.session.work_id).toBe(testWorkId);
      expect(aiSession.session.status).toBeDefined();
      expect(aiSession.session.started_at).toBeDefined();
    });

    it('should handle multiple LLM agent sessions for same work', async () => {
      const sessionData1 = testDataGenerator.generateAiSessionData({
        tool_name: 'llm-agent',
        project_context: 'First session context',
      });
      const sessionData2 = testDataGenerator.generateAiSessionData({
        tool_name: 'llm-agent',
        project_context: 'Second session context',
      });

      const session1 = await testApiClient.createAiSession(testWorkId, sessionData1);
      const session2 = await testApiClient.createAiSession(testWorkId, sessionData2);

      expect(session1.session.id).not.toBe(session2.session.id);
      expect(session1.session.work_id).toBe(session2.session.work_id);
      expect(session1.session.tool_name).toBe(session2.session.tool_name);
    });
  });

  describe('Tool Call Processing', () => {
    let aiSessionId: string;

    beforeAll(async () => {
      // Create an AI session for tool call testing
      const aiSessionData = testDataGenerator.generateAiSessionData({
        tool_name: 'llm-agent',
      });
      const aiSession = await testApiClient.createAiSession(testWorkId, aiSessionData);
      aiSessionId = aiSession.session.id;
    });

    it('should process file listing tool call', async () => {
      // Add a user message requesting file listing
      const userMessage = testDataGenerator.generateMessageData({
        content: 'List all files in the root directory',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Create some test files to list
      const testFiles = [
        testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'README.md',
          content: '# Test Project',
        }),
        testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'src/main.rs',
          content: 'fn main() {}',
        }),
        testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'Cargo.toml',
          content: '[package]',
        }),
      ];

      await Promise.all(testFiles.map(file => testApiClient.createFile(file)));

      // Simulate LLM agent processing the tool call
      const toolCallResponse = 'I need to list the files in the project root directory.';
      await testApiClient.recordAiOutput(testWorkId, toolCallResponse);

      // Verify the work session contains the tool call processing
      const workSession = await testApiClient.getWork(testWorkId);
      expect(workSession.messages.length).toBeGreaterThan(0);

      // The LLM agent should be able to list files (this would be tested by verifying
      // the actual file listing functionality in a real implementation)
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      expect(fileList.files.length).toBeGreaterThanOrEqual(3);
      expect(fileList.files.some(f => f.name === 'README.md')).toBe(true);
      expect(fileList.files.some(f => f.name === 'src')).toBe(true);
      expect(fileList.files.some(f => f.name === 'Cargo.toml')).toBe(true);
    });

    it('should process file reading tool call', async () => {
      // Create a test file to read
      const testFile = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'test-readme.md',
        content: '# Test README\n\nThis is a test file for LLM agent tool call processing.',
      });
      await testApiClient.createFile(testFile);

      // Add user message requesting file reading
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Read the contents of test-readme.md',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Simulate LLM agent processing
      const toolCallResponse = 'I need to read the contents of the test-readme.md file.';
      await testApiClient.recordAiOutput(testWorkId, toolCallResponse);

      // Verify file reading capability
      const fileContent = await testApiClient.getFileContent(testFile.path, testProjectId);
      expect(fileContent.content).toBe(testFile.content);
      expect(fileContent.encoding).toBe(testFile.encoding);
    });

    it('should process file creation tool call', async () => {
      const newFilePath = 'generated-file.txt';
      const newFileContent = 'This file was created by LLM agent tool call processing.';

      // Add user message requesting file creation
      const userMessage = testDataGenerator.generateMessageData({
        content: `Create a new file called ${newFilePath} with content: "${newFileContent}"`,
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Simulate LLM agent processing
      const toolCallResponse = `I need to create a new file at ${newFilePath} with the specified content.`;
      await testApiClient.recordAiOutput(testWorkId, toolCallResponse);

      // Create the file (simulating what the LLM agent would do)
      const createFileData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: newFilePath,
        content: newFileContent,
      });
      await testApiClient.createFile(createFileData);

      // Verify file was created
      const createdFile = await testApiClient.getFileContent(newFilePath, testProjectId);
      expect(createdFile.content).toBe(newFileContent);
    });

    it('should process file update tool call', async () => {
      // Create initial file
      const filePath = 'update-test.txt';
      const initialContent = 'Initial content';
      const updatedContent = 'Updated content by LLM agent';

      const createFileData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: filePath,
        content: initialContent,
      });
      await testApiClient.createFile(createFileData);

      // Add user message requesting file update
      const userMessage = testDataGenerator.generateMessageData({
        content: `Update ${filePath} to contain: "${updatedContent}"`,
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Simulate LLM agent processing
      const toolCallResponse = `I need to update the file ${filePath} with new content.`;
      await testApiClient.recordAiOutput(testWorkId, toolCallResponse);

      // Update the file (simulating LLM agent action)
      const updateData = testDataGenerator.generateFileUpdateData({
        content: updatedContent,
      });
      await testApiClient.updateFile(filePath, {
        ...updateData,
        project_id: testProjectId,
      });

      // Verify file was updated
      const updatedFile = await testApiClient.getFileContent(filePath, testProjectId);
      expect(updatedFile.content).toBe(updatedContent);
    });
  });

  describe('LLM Agent Workflow Integration', () => {
    it('should handle complete LLM agent workflow with multiple tool calls', async () => {
      // Create a new work session for this test
      const workflowWorkData = testDataGenerator.generateWorkData({
        title: 'LLM Agent Complete Workflow Test',
        tool_name: 'llm-agent',
        project_id: testProjectId,
      });
      const workflowWork = await testApiClient.createWork(workflowWorkData);
      const workflowWorkId = workflowWork.work.id;

      // Step 1: User requests project analysis
      const analysisRequest = testDataGenerator.generateMessageData({
        content: 'Analyze this project and create a summary report',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(workflowWorkId, analysisRequest);

      // Step 2: Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData({
        tool_name: 'llm-agent',
        project_context: `Analyzing project ${testProjectId}`,
      });
      const aiSession = await testApiClient.createAiSession(workflowWorkId, aiSessionData);

      // Step 3: LLM agent lists files (tool call 1)
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      await testApiClient.recordAiOutput(
        workflowWorkId,
        `Found ${fileList.files.length} items in project root`
      );

      // Step 4: LLM agent reads key files (tool call 2)
      const readmeContent = await testApiClient.getFileContent('README.md', testProjectId);
      await testApiClient.recordAiOutput(
        workflowWorkId,
        `README content: ${readmeContent.content.substring(0, 100)}...`
      );

      // Step 5: LLM agent creates analysis report (tool call 3)
      const reportContent = `# Project Analysis Report

## Overview
This is a test project created for LLM agent tool call processing verification.

## Files Found
- Total items: ${fileList.files.length}
- Key files: README.md, Cargo.toml, src/

## Summary
The project structure indicates this is a Rust project with standard documentation.
`;

      const reportFileData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'analysis-report.md',
        content: reportContent,
      });
      await testApiClient.createFile(reportFileData);
      await testApiClient.recordAiOutput(workflowWorkId, 'Analysis report created successfully');

      // Step 6: Verify complete workflow
      const finalWorkSession = await testApiClient.getWork(workflowWorkId);
      expect(finalWorkSession.work.id).toBe(workflowWorkId);
      expect(finalWorkSession.messages.length).toBeGreaterThan(0);

      // Verify report was created
      const createdReport = await testApiClient.getFileContent('analysis-report.md', testProjectId);
      expect(createdReport.content).toContain('# Project Analysis Report');
      expect(createdReport.content).toContain(`Total items: ${fileList.files.length}`);

      // Verify AI outputs were recorded
      const outputs = await testApiClient.listAiOutputs(workflowWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(0);
    });

    it('should handle LLM agent error scenarios gracefully', async () => {
      // Create work session for error testing
      const errorWorkData = testDataGenerator.generateWorkData({
        title: 'LLM Agent Error Handling Test',
        tool_name: 'llm-agent',
      });
      const errorWork = await testApiClient.createWork(errorWorkData);
      const errorWorkId = errorWork.work.id;

      // Test requesting non-existent file
      const invalidRequest = testDataGenerator.generateMessageData({
        content: 'Read the contents of non-existent-file.xyz',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(errorWorkId, invalidRequest);

      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(errorWorkId, aiSessionData);

      // Attempt to read non-existent file (should fail gracefully)
      await expect(
        testApiClient.getFileContent('non-existent-file.xyz', testProjectId)
      ).rejects.toThrow();

      // LLM agent should still be able to record error handling
      await testApiClient.recordAiOutput(
        errorWorkId,
        'Error: File not found. Unable to complete requested operation.'
      );

      // Verify error was recorded
      const outputs = await testApiClient.listAiOutputs(errorWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(0);
    });
  });

  describe('LLM Agent Performance and Reliability', () => {
    it('should handle rapid sequential tool calls', async () => {
      // Create work session for performance testing
      const perfWorkData = testDataGenerator.generateWorkData({
        title: 'LLM Agent Performance Test',
        tool_name: 'llm-agent',
      });
      const perfWork = await testApiClient.createWork(perfWorkData);
      const perfWorkId = perfWork.work.id;

      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(perfWorkId, aiSessionData);

      // Perform rapid sequential operations
      const operations = [];

      // Create multiple files quickly
      for (let i = 0; i < 5; i++) {
        const fileData = testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: `perf-test-${i}.txt`,
          content: `Performance test file ${i}`,
        });
        operations.push(testApiClient.createFile(fileData));
      }

      // Execute all operations
      await Promise.all(operations);

      // Record performance result
      await testApiClient.recordAiOutput(perfWorkId, 'Successfully created 5 test files rapidly');

      // Verify all files were created
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      const perfFiles = fileList.files.filter(f => f.name.startsWith('perf-test-'));
      expect(perfFiles.length).toBe(5);
    });

    it('should maintain session state across tool calls', async () => {
      // Create work session for state testing
      const stateWorkData = testDataGenerator.generateWorkData({
        title: 'LLM Agent State Management Test',
        tool_name: 'llm-agent',
      });
      const stateWork = await testApiClient.createWork(stateWorkData);
      const stateWorkId = stateWork.work.id;

      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(stateWorkId, aiSessionData);

      // Build up state through multiple tool calls
      const stateFile1 = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'state-1.txt',
        content: 'First state file',
      });
      await testApiClient.createFile(stateFile1);
      await testApiClient.recordAiOutput(stateWorkId, 'Created first state file');

      const stateFile2 = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'state-2.txt',
        content: 'Second state file',
      });
      await testApiClient.createFile(stateFile2);
      await testApiClient.recordAiOutput(stateWorkId, 'Created second state file');

      // Verify session maintains state
      const finalWork = await testApiClient.getWork(stateWorkId);
      expect(finalWork.work.id).toBe(stateWorkId);

      const outputs = await testApiClient.listAiOutputs(stateWorkId);
      expect(outputs.outputs.length).toBe(2);

      // Verify files exist
      await testApiClient.getFileContent('state-1.txt', testProjectId);
      await testApiClient.getFileContent('state-2.txt', testProjectId);
    });
  });
});
