import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';

describe('File Operations - API Only', () => {
  let testProjectId: string;

  beforeAll(async () => {
    await testDatabase.setupTestDatabase();
    await testServer.startServer();

    // Create a test project for file operations
    const projectData = testDataGenerator.generateProjectData();
    const project = await testApiClient.createProject(projectData);
    testProjectId = project.id;
  }, 30000);

  afterAll(async () => {
    // Clean up test project
    if (testProjectId) {
      await testApiClient.deleteProject(testProjectId);
    }

    await testServer.stopServer();
    await testDatabase.cleanupTestDatabase();
  });

  beforeEach(async () => {
    testDataGenerator.reset();
  });

  describe('File CRUD Operations', () => {
    it('should create a new file', async () => {
      const fileData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'test-file.txt',
        content: 'Hello, World!',
      });

      const createdFile = await testApiClient.createFile(fileData);

      expect(createdFile).toBeDefined();
      expect(createdFile.path).toBe(fileData.path);
      expect(createdFile.project_id).toBe(testProjectId);
    });

    it('should read file content', async () => {
      const fileData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'readme.md',
        content: '# Test README\n\nThis is a test file.',
      });

      // Create the file
      await testApiClient.createFile(fileData);

      // Read the file content
      const fileContent = await testApiClient.getFileContent(fileData.path, testProjectId);

      expect(fileContent).toBeDefined();
      expect(fileContent.content).toBe(fileData.content);
      expect(fileContent.encoding).toBe(fileData.encoding);
    });

    it('should update file content', async () => {
      const fileData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'updatable-file.txt',
        content: 'Original content',
      });

      // Create the file
      await testApiClient.createFile(fileData);

      // Update the file
      const updateData = testDataGenerator.generateFileUpdateData({
        content: 'Updated content',
      });

      const updatedFile = await testApiClient.updateFile(fileData.path, {
        ...updateData,
        project_id: testProjectId,
      });

      expect(updatedFile).toBeDefined();
      expect(updatedFile.content).toBe(updateData.content);

      // Verify the update by reading the file
      const fileContent = await testApiClient.getFileContent(fileData.path, testProjectId);
      expect(fileContent.content).toBe(updateData.content);
    });

    it('should delete a file', async () => {
      const fileData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'deletable-file.txt',
      });

      // Create the file
      await testApiClient.createFile(fileData);

      // Delete the file
      await testApiClient.deleteFile(fileData.path, testProjectId);

      // Verify file is deleted
      await expect(testApiClient.getFileContent(fileData.path, testProjectId)).rejects.toThrow();
    });
  });

  describe('File Listing', () => {
    beforeAll(async () => {
      // Create some test files
      const files = [
        testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'src/main.rs',
          content: 'fn main() { println!("Hello"); }',
        }),
        testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'src/lib.rs',
          content: 'pub fn test() {}',
        }),
        testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'README.md',
          content: '# Project README',
        }),
        testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: 'Cargo.toml',
          content: '[package]\nname = "test"',
        }),
      ];

      await Promise.all(files.map(file => testApiClient.createFile(file)));
    });

    it('should list all files in project', async () => {
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });

      expect(fileList).toBeDefined();
      expect(Array.isArray(fileList.files)).toBe(true);
      expect(fileList.files.length).toBeGreaterThanOrEqual(4);

      // Check that our test files are present
      const fileNames = fileList.files.map(f => f.name);
      expect(fileNames).toContain('src');
      expect(fileNames).toContain('README.md');
      expect(fileNames).toContain('Cargo.toml');
    });

    it('should list files in specific directory', async () => {
      const fileList = await testApiClient.listFiles({
        project_id: testProjectId,
        path: 'src',
      });

      expect(fileList).toBeDefined();
      expect(Array.isArray(fileList.files)).toBe(true);

      // Should contain main.rs and lib.rs
      const fileNames = fileList.files.map(f => f.name);
      expect(fileNames).toContain('main.rs');
      expect(fileNames).toContain('lib.rs');
    });

    it('should handle empty directory listing', async () => {
      const fileList = await testApiClient.listFiles({
        project_id: testProjectId,
        path: 'non-existent-dir',
      });

      expect(fileList).toBeDefined();
      expect(Array.isArray(fileList.files)).toBe(true);
      // Should be empty or not found
    });
  });

  describe('File Validation', () => {
    it('should reject invalid file creation data', async () => {
      const invalidFileData = testDataGenerator.generateErrorScenarios().invalidFile;

      await expect(testApiClient.createFile(invalidFileData)).rejects.toThrow();
    });

    it('should handle reading non-existent file', async () => {
      await expect(
        testApiClient.getFileContent('non-existent-file.txt', testProjectId)
      ).rejects.toThrow();
    });

    it('should handle updating non-existent file', async () => {
      const updateData = testDataGenerator.generateFileUpdateData();

      await expect(
        testApiClient.updateFile('non-existent-file.txt', {
          ...updateData,
          project_id: testProjectId,
        })
      ).rejects.toThrow();
    });

    it('should handle deleting non-existent file', async () => {
      await expect(
        testApiClient.deleteFile('non-existent-file.txt', testProjectId)
      ).rejects.toThrow();
    });
  });

  describe('File Operations Workflow', () => {
    it('should support complete file lifecycle', async () => {
      const filePath = 'lifecycle-test.txt';
      const initialContent = 'Initial content';
      const updatedContent = 'Updated content';

      // Create file
      const createData = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: filePath,
        content: initialContent,
      });

      const createdFile = await testApiClient.createFile(createData);
      expect(createdFile.path).toBe(filePath);

      // Read file
      let fileContent = await testApiClient.getFileContent(filePath, testProjectId);
      expect(fileContent.content).toBe(initialContent);

      // Update file
      const updateData = testDataGenerator.generateFileUpdateData({
        content: updatedContent,
      });

      await testApiClient.updateFile(filePath, {
        ...updateData,
        project_id: testProjectId,
      });

      // Verify update
      fileContent = await testApiClient.getFileContent(filePath, testProjectId);
      expect(fileContent.content).toBe(updatedContent);

      // Delete file
      await testApiClient.deleteFile(filePath, testProjectId);

      // Verify deletion
      await expect(testApiClient.getFileContent(filePath, testProjectId)).rejects.toThrow();
    });
  });
});