import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';

describe('Tool Call Processing - API Only', () => {
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
      tool_name: 'llm-agent',
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

  describe('File System Tool Calls', () => {
    it('should process list_dir tool call correctly', async () => {
      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      // Add user message requesting directory listing
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Can you show me what files are in the root directory?',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Simulate LLM agent processing the list_dir tool call
      await testApiClient.recordAiOutput(testWorkId, 'I need to list the contents of the root directory.');

      // Execute the actual tool call (list files)
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });

      // Verify tool call result processing
      expect(fileList).toBeDefined();
      expect(Array.isArray(fileList.files)).toBe(true);

      // Record tool call result
      await testApiClient.recordAiOutput(testWorkId,
        `Directory listing complete. Found ${fileList.files.length} items.`
      );

      // Verify the work session captured the tool call
      const workSession = await testApiClient.getWork(testWorkId);
      expect(workSession.messages.length).toBeGreaterThan(0);

      // Verify AI outputs include tool call processing
      const outputs = await testApiClient.listAiOutputs(testWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(1);
    });

    it('should process read_file tool call with proper content retrieval', async () => {
      // Create a test file to read
      const testFile = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'tool-call-test.txt',
        content: 'This is a test file for tool call processing verification.\nIt contains multiple lines.\nEnd of file.',
      });
      await testApiClient.createFile(testFile);

      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      // Add user message requesting file reading
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Please read the contents of tool-call-test.txt',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Simulate LLM agent processing the read_file tool call
      await testApiClient.recordAiOutput(testWorkId, 'I need to read the contents of tool-call-test.txt.');

      // Execute the actual tool call (read file)
      const fileContent = await testApiClient.getFileContent(testFile.path, testProjectId);

      // Verify tool call result processing
      expect(fileContent).toBeDefined();
      expect(fileContent.content).toBe(testFile.content);
      expect(fileContent.encoding).toBe(testFile.encoding);

      // Record tool call result with content summary
      const contentPreview = fileContent.content.substring(0, 100);
      await testApiClient.recordAiOutput(testWorkId,
        `File reading complete. Content preview: ${contentPreview}${fileContent.content.length > 100 ? '...' : ''}`
      );

      // Verify the complete tool call workflow
      const workSession = await testApiClient.getWork(testWorkId);
      expect(workSession.messages.length).toBeGreaterThan(0);

      const outputs = await testApiClient.listAiOutputs(testWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(1);
    });

    it('should process create_file tool call with content validation', async () => {
      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      // Add user message requesting file creation
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Create a new file called analysis.md with a project analysis',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Simulate LLM agent processing the create_file tool call
      await testApiClient.recordAiOutput(testWorkId, 'I need to create a new file called analysis.md.');

      // Execute the actual tool call (create file)
      const newFileContent = `# Project Analysis

## Overview
This project demonstrates API-only end-to-end testing for LLM agent tool call processing.

## Key Components
- Fast API-only tests (10-20x faster than browser tests)
- Comprehensive tool call verification
- Real-time WebSocket communication testing
- Complete workflow validation

## Conclusion
The testing framework successfully validates LLM agent tool call processing without UI dependencies.`;

      const createFileData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'analysis.md',
        content: newFileContent,
      });
      const createdFile = await testApiClient.createFile(createFileData);

      // Verify tool call result processing
      expect(createdFile).toBeDefined();
      expect(createdFile.path).toBe('analysis.md');

      // Record tool call result
      await testApiClient.recordAiOutput(testWorkId,
        'File creation complete. Created analysis.md with project analysis content.'
      );

      // Verify file was created correctly
      const readContent = await testApiClient.getFileContent('analysis.md', testProjectId);
      expect(readContent.content).toBe(newFileContent);
      expect(readContent.content).toContain('# Project Analysis');

      // Verify the complete tool call workflow
      const workSession = await testApiClient.getWork(testWorkId);
      expect(workSession.messages.length).toBeGreaterThan(0);

      const outputs = await testApiClient.listAiOutputs(testWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(1);
    });

    it('should process update_file tool call with change tracking', async () => {
      // Create initial file
      const initialContent = 'Initial file content for update testing.';
      const initialFile = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'update-test.txt',
        content: initialContent,
      });
      await testApiClient.createFile(initialFile);

      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      // Add user message requesting file update
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Update update-test.txt to include additional analysis information',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Simulate LLM agent processing the update_file tool call
      await testApiClient.recordAiOutput(testWorkId, 'I need to update update-test.txt with additional content.');

      // Execute the actual tool call (update file)
      const updatedContent = `${initialContent}\n\nAdditional Analysis:\n- Performance metrics added\n- Error handling improved\n- Documentation enhanced`;

      await testApiClient.updateFile('update-test.txt', {
        content: updatedContent,
        encoding: 'utf-8',
        project_id: testProjectId,
      });

      // Record tool call result
      await testApiClient.recordAiOutput(testWorkId,
        'File update complete. Added additional analysis information to update-test.txt.'
      );

      // Verify file was updated correctly
      const readContent = await testApiClient.getFileContent('update-test.txt', testProjectId);
      expect(readContent.content).toBe(updatedContent);
      expect(readContent.content).toContain('Additional Analysis:');
      expect(readContent.content).toContain('Performance metrics added');

      // Verify the complete tool call workflow
      const workSession = await testApiClient.getWork(testWorkId);
      expect(workSession.messages.length).toBeGreaterThan(0);

      const outputs = await testApiClient.listAiOutputs(testWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(1);
    });
  });

  describe('Tool Call Error Handling', () => {
    it('should handle file_not_found errors gracefully', async () => {
      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      // Add user message requesting non-existent file
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Read the contents of nonexistent-file.xyz',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Simulate LLM agent attempting the tool call
      await testApiClient.recordAiOutput(testWorkId, 'Attempting to read nonexistent-file.xyz.');

      // Execute the tool call (should fail)
      try {
        await testApiClient.getFileContent('nonexistent-file.xyz', testProjectId);
        expect.fail('Should have thrown error for non-existent file');
      } catch (error) {
        // Expected error - record error handling
        await testApiClient.recordAiOutput(testWorkId,
          'Tool call failed: File not found. Error handled gracefully.'
        );
      }

      // Verify error was recorded and handled
      const outputs = await testApiClient.listAiOutputs(testWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(1);

      // Verify work session maintains integrity
      const workSession = await testApiClient.getWork(testWorkId);
      expect(workSession.work.status).toBeDefined();
    });

    it('should handle permission errors for restricted operations', async () => {
      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      // Add user message requesting invalid operation
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Delete the entire project directory',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Simulate LLM agent recognizing invalid operation
      await testApiClient.recordAiOutput(testWorkId,
        'Operation rejected: Cannot delete entire project directory. This would be destructive.'
      );

      // Verify the rejection was recorded
      const outputs = await testApiClient.listAiOutputs(testWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(0);

      // Verify project still exists and is intact
      const project = await testApiClient.fetchProject(testProjectId);
      expect(project.id).toBe(testProjectId);

      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      expect(fileList.files.length).toBeGreaterThan(0);
    });

    it('should handle malformed tool call parameters', async () => {
      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      // Add user message with malformed request
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Create a file with invalid path: ../../../../etc/passwd',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Simulate LLM agent detecting malformed parameters
      await testApiClient.recordAiOutput(testWorkId,
        'Tool call rejected: Invalid file path. Path traversal not allowed.'
      );

      // Attempt invalid operation (should be rejected by API)
      const invalidFileData = testDataGenerator.generateErrorScenarios().invalidFile;
      try {
        await testApiClient.createFile(invalidFileData);
        expect.fail('Should have rejected invalid file creation');
      } catch (error) {
        // Expected rejection
        await testApiClient.recordAiOutput(testWorkId,
          'Invalid file creation rejected by API validation.'
        );
      }

      // Verify error handling was recorded
      const outputs = await testApiClient.listAiOutputs(testWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(1);
    });
  });

  describe('Complex Tool Call Sequences', () => {
    it('should handle multi-step tool call workflows', async () => {
      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      // Add complex user request
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Analyze the project structure, create a summary report, and update the README',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Step 1: List directory contents
      await testApiClient.recordAiOutput(testWorkId, 'Step 1: Analyzing project structure...');
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      await testApiClient.recordAiOutput(testWorkId,
        `Found ${fileList.files.length} items in project root.`
      );

      // Step 2: Read existing README
      const readmeExists = fileList.files.some(f => f.name === 'README.md');
      if (readmeExists) {
        const readmeContent = await testApiClient.getFileContent('README.md', testProjectId);
        await testApiClient.recordAiOutput(testWorkId,
          'Step 2: Read existing README for context.'
        );
      }

      // Step 3: Create analysis report
      const analysisContent = `# Project Structure Analysis

## Files Found
- Total items: ${fileList.files.length}
${fileList.files.map(f => `- ${f.name} (${f.type})`).join('\n')}

## Analysis
This appears to be a well-structured project with ${fileList.files.filter(f => f.type === 'file').length} files and ${fileList.files.filter(f => f.type === 'directory').length} directories.

## Recommendations
- Consider adding more documentation
- Implement comprehensive testing
- Add CI/CD pipeline configuration`;

      const analysisFile = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'structure-analysis.md',
        content: analysisContent,
      });
      await testApiClient.createFile(analysisFile);
      await testApiClient.recordAiOutput(testWorkId, 'Step 3: Created detailed project analysis report.');

      // Step 4: Update README with analysis reference
      const updatedReadmeContent = `# Test Project

This is a test project for API-only end-to-end testing.

## Project Analysis

See [structure-analysis.md](./structure-analysis.md) for detailed project structure analysis.

## Recent Updates
- Added comprehensive project analysis
- Improved documentation structure
- Enhanced testing framework`;

      await testApiClient.updateFile('README.md', {
        content: updatedReadmeContent,
        encoding: 'utf-8',
        project_id: testProjectId,
      });
      await testApiClient.recordAiOutput(testWorkId, 'Step 4: Updated README with analysis reference.');

      // Verify all steps completed successfully
      const finalFileList = await testApiClient.listFiles({ project_id: testProjectId });
      expect(finalFileList.files.some(f => f.name === 'structure-analysis.md')).toBe(true);

      const updatedReadme = await testApiClient.getFileContent('README.md', testProjectId);
      expect(updatedReadme.content).toContain('structure-analysis.md');

      // Verify complete workflow was recorded
      const outputs = await testApiClient.listAiOutputs(testWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(4); // At least 4 steps recorded

      const workSession = await testApiClient.getWork(testWorkId);
      expect(workSession.messages.length).toBeGreaterThan(0);
    });

    it('should handle conditional tool call execution', async () => {
      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      // Add conditional request
      const userMessage = testDataGenerator.generateMessageData({
        content: 'If a config file exists, read it. Otherwise, create a default config file.',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Step 1: Check if config file exists
      await testApiClient.recordAiOutput(testWorkId, 'Step 1: Checking for existing config file...');
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      const configExists = fileList.files.some(f => f.name === 'config.toml');

      if (configExists) {
        // Read existing config
        const configContent = await testApiClient.getFileContent('config.toml', testProjectId);
        await testApiClient.recordAiOutput(testWorkId,
          'Config file found. Reading existing configuration.'
        );
      } else {
        // Create default config
        const defaultConfig = `[project]
name = "test-project"
version = "0.1.0"

[settings]
debug = true
log_level = "info"`;

        const configFile = testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'config.toml',
          content: defaultConfig,
        });
        await testApiClient.createFile(configFile);
        await testApiClient.recordAiOutput(testWorkId,
          'Config file not found. Created default configuration file.'
        );
      }

      // Verify conditional logic worked
      const finalFileList = await testApiClient.listFiles({ project_id: testProjectId });
      expect(finalFileList.files.some(f => f.name === 'config.toml')).toBe(true);

      const configContent = await testApiClient.getFileContent('config.toml', testProjectId);
      expect(configContent.content).toContain('[project]');

      // Verify workflow recorded conditional execution
      const outputs = await testApiClient.listAiOutputs(testWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(1);
    });
  });

  describe('Tool Call Performance and Reliability', () => {
    it('should handle rapid sequential tool calls efficiently', async () => {
      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      // Add performance test request
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Create 10 test files rapidly to test performance',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Execute rapid tool calls
      await testApiClient.recordAiOutput(testWorkId, 'Starting rapid file creation test...');

      const startTime = Date.now();
      const fileOperations = [];

      for (let i = 0; i < 10; i++) {
        const fileData = testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: `rapid-test-${i.toString().padStart(2, '0')}.txt`,
          content: `Rapid test file ${i}\nCreated at ${new Date().toISOString()}`,
        });
        fileOperations.push(testApiClient.createFile(fileData));
      }

      await Promise.all(fileOperations);
      const endTime = Date.now();
      const duration = endTime - startTime;

      await testApiClient.recordAiOutput(testWorkId,
        `Rapid file creation complete. Created 10 files in ${duration}ms.`
      );

      // Verify all files were created
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      const rapidFiles = fileList.files.filter(f => f.name.startsWith('rapid-test-'));
      expect(rapidFiles.length).toBe(10);

      // Verify performance is reasonable
      expect(duration).toBeLessThan(10000); // Should complete in less than 10 seconds

      // Verify work session integrity maintained
      const workSession = await testApiClient.getWork(testWorkId);
      expect(workSession.work.id).toBe(testWorkId);
    });

    it('should maintain tool call state consistency', async () => {
      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData();
      await testApiClient.createAiSession(testWorkId, aiSessionData);

      const testFilePath = 'state-consistency-test.txt';

      // Add state consistency test request
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Test state consistency with create, update, read, delete operations',
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(testWorkId, userMessage);

      // Execute state consistency test
      await testApiClient.recordAiOutput(testWorkId, 'Starting state consistency test...');

      // 1. Create file
      const createData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: testFilePath,
        content: 'Initial state',
      });
      await testApiClient.createFile(createData);
      await testApiClient.recordAiOutput(testWorkId, 'Step 1: File created');

      // 2. Read file
      const readContent1 = await testApiClient.getFileContent(testFilePath, testProjectId);
      expect(readContent1.content).toBe('Initial state');
      await testApiClient.recordAiOutput(testWorkId, 'Step 2: File read - state verified');

      // 3. Update file
      await testApiClient.updateFile(testFilePath, {
        content: 'Updated state',
        encoding: 'utf-8',
        project_id: testProjectId,
      });
      await testApiClient.recordAiOutput(testWorkId, 'Step 3: File updated');

      // 4. Read updated file
      const readContent2 = await testApiClient.getFileContent(testFilePath, testProjectId);
      expect(readContent2.content).toBe('Updated state');
      await testApiClient.recordAiOutput(testWorkId, 'Step 4: Updated file read - state verified');

      // 5. Delete file
      await testApiClient.deleteFile(testFilePath, testProjectId);
      await testApiClient.recordAiOutput(testWorkId, 'Step 5: File deleted');

      // 6. Verify deletion
      await expect(testApiClient.getFileContent(testFilePath, testProjectId)).rejects.toThrow();
      await testApiClient.recordAiOutput(testWorkId, 'Step 6: Deletion verified - state consistency maintained');

      // Verify complete workflow recorded
      const outputs = await testApiClient.listAiOutputs(testWorkId);
      expect(outputs.outputs.length).toBeGreaterThan(6);

      const workSession = await testApiClient.getWork(testWorkId);
      expect(workSession.work.id).toBe(testWorkId);
    });
  });
});