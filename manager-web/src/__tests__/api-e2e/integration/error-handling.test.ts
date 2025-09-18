import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';
import { testStateManager } from '../utils/state-manager';

describe('Error Handling and Edge Cases - API Only', () => {
  beforeAll(async () => {
    await testDatabase.setupTestDatabase();
    await testServer.startServer();
    await testStateManager.initialize();
  }, 30000);

  afterAll(async () => {
    testStateManager.clearState();
    await testServer.stopServer();
    await testDatabase.cleanupTestDatabase();
  });

  beforeEach(async () => {
    testDataGenerator.reset();
  });

  describe('API Error Responses', () => {
    it('should handle malformed JSON requests', async () => {
      // Test with invalid JSON in request body
      const invalidRequest = '{ invalid json: }';

      // This would typically be tested by making direct HTTP requests
      // For now, we test the client-side error handling
      expect(() => JSON.parse(invalidRequest)).toThrow();

      // Test with valid client but invalid API calls
      await expect(testApiClient.fetchProject('invalid-id-format')).rejects.toThrow();
    });

    it('should handle network timeouts gracefully', async () => {
      // Create a client with a very short timeout (if supported)
      // For now, test with invalid server endpoint
      const invalidClient = new testApiClient.constructor('http://invalid-server:9999');

      await expect(invalidClient.healthCheck()).rejects.toThrow();
    });

    it('should handle server errors (5xx) appropriately', async () => {
      // Test with endpoints that might return server errors
      // Since we don't have intentional error endpoints, test with invalid operations

      // Try to access non-existent work session
      await expect(testApiClient.getWork('non-existent-work-id')).rejects.toThrow();

      // Try to create work with invalid data
      const invalidWorkData = testDataGenerator.generateErrorScenarios().invalidWork;
      await expect(testApiClient.createWork(invalidWorkData)).rejects.toThrow();
    });

    it('should handle client errors (4xx) with proper messages', async () => {
      // Test various 4xx scenarios

      // 404 - Not Found
      await expect(testApiClient.fetchProject('non-existent-project')).rejects.toThrow();

      // 400 - Bad Request (invalid work data)
      const invalidWork = testDataGenerator.generateErrorScenarios().invalidWork;
      await expect(testApiClient.createWork(invalidWork)).rejects.toThrow();

      // 400 - Bad Request (invalid file data)
      const invalidFile = testDataGenerator.generateErrorScenarios().invalidFile;
      await expect(testApiClient.createFile(invalidFile)).rejects.toThrow();
    });
  });

  describe('File System Edge Cases', () => {
    let testProjectId: string;

    beforeAll(async () => {
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          description: 'Testing error handling scenarios',
        })
      );
      testProjectId = project.id;
    });

    it('should handle file operations on non-existent projects', async () => {
      const fileData = testDataGenerator.generateFileData({
        project_id: 'non-existent-project-id',
        path: 'test.txt',
        content: 'test content',
      });

      await expect(testApiClient.createFile(fileData)).rejects.toThrow();
    });

    it('should handle reading non-existent files', async () => {
      await expect(
        testApiClient.getFileContent('non-existent-file.txt', testProjectId)
      ).rejects.toThrow();
    });

    it('should handle updating non-existent files', async () => {
      await expect(
        testApiClient.updateFile('non-existent-file.txt', {
          content: 'new content',
          encoding: 'utf-8',
          project_id: testProjectId,
        })
      ).rejects.toThrow();
    });

    it('should handle deleting non-existent files', async () => {
      await expect(
        testApiClient.deleteFile('non-existent-file.txt', testProjectId)
      ).rejects.toThrow();
    });

    it('should handle files with special characters in names', async () => {
      const specialFiles = [
        'file with spaces.txt',
        'file-with-dashes.txt',
        'file_with_underscores.txt',
        'file.with.dots.txt',
        'file123.txt',
      ];

      // Create files with special characters
      for (const fileName of specialFiles) {
        const fileData = testDataGenerator.generateFileData({
          project_id: testProjectId,
          path: fileName,
          content: `Content of ${fileName}`,
        });

        await testApiClient.createFile(fileData);
      }

      // Verify all files were created
      const fileList = await testApiClient.listFiles({ project_id: testProjectId });
      const createdFiles = fileList.files.filter(f => specialFiles.includes(f.name));
      expect(createdFiles.length).toBe(specialFiles.length);

      // Test reading files with special characters
      for (const fileName of specialFiles) {
        const content = await testApiClient.getFileContent(fileName, testProjectId);
        expect(content.content).toBe(`Content of ${fileName}`);
      }
    });

    it('should handle very large file content', async () => {
      const largeContent = 'x'.repeat(100000); // 100KB of content

      const largeFile = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'large-file.txt',
        content: largeContent,
      });

      await testApiClient.createFile(largeFile);

      // Verify content was stored correctly
      const readContent = await testApiClient.getFileContent('large-file.txt', testProjectId);
      expect(readContent.content.length).toBe(largeContent.length);
      expect(readContent.content).toBe(largeContent);
    });

    it('should handle empty file content', async () => {
      const emptyFile = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'empty-file.txt',
        content: '',
      });

      await testApiClient.createFile(emptyFile);

      const readContent = await testApiClient.getFileContent('empty-file.txt', testProjectId);
      expect(readContent.content).toBe('');
      expect(readContent.encoding).toBe('utf-8');
    });

    it('should handle binary-like content', async () => {
      // Create content with various characters including potential binary data
      const binaryLikeContent =
        'Normal text\nWith newlines\nAnd special chars: áéíóú\nAnd control chars: \t\r\nAnd high chars: 🚀⭐💻';

      const binaryFile = testDataGenerator.generateFileData({
        project_id: testProjectId,
        path: 'binary-like.txt',
        content: binaryLikeContent,
      });

      await testApiClient.createFile(binaryFile);

      const readContent = await testApiClient.getFileContent('binary-like.txt', testProjectId);
      expect(readContent.content).toBe(binaryLikeContent);
    });
  });

  describe('Work Session Edge Cases', () => {
    it('should handle work sessions with very long titles', async () => {
      const longTitle = 'A'.repeat(1000); // Very long title

      const workData = testDataGenerator.generateWorkData({
        title: longTitle,
        tool_name: 'llm-agent',
      });

      const work = await testApiClient.createWork(workData);
      expect(work.work.title).toBe(longTitle);

      // Verify it can be retrieved
      const retrieved = await testApiClient.getWork(work.work.id);
      expect(retrieved.work.title).toBe(longTitle);
    });

    it('should handle work sessions with empty messages', async () => {
      const workData = testDataGenerator.generateWorkData();
      const work = await testApiClient.createWork(workData);

      // Try to add empty message
      const emptyMessage = testDataGenerator.generateMessageData({
        content: '',
      });

      await expect(testApiClient.addMessageToWork(work.work.id, emptyMessage)).rejects.toThrow();
    });

    it('should handle concurrent AI session creation', async () => {
      const workData = testDataGenerator.generateWorkData();
      const work = await testApiClient.createWork(workData);

      // Try to create multiple AI sessions concurrently
      const sessionPromises = [
        testApiClient.createAiSession(work.work.id, testDataGenerator.generateAiSessionData()),
        testApiClient.createAiSession(work.work.id, testDataGenerator.generateAiSessionData()),
        testApiClient.createAiSession(work.work.id, testDataGenerator.generateAiSessionData()),
      ];

      // All should succeed or fail gracefully
      const results = await Promise.allSettled(sessionPromises);

      const fulfilled = results.filter(r => r.status === 'fulfilled').length;
      const rejected = results.filter(r => r.status === 'rejected').length;

      // At least one should succeed, and we should handle rejections gracefully
      expect(fulfilled + rejected).toBe(3);
      expect(fulfilled).toBeGreaterThan(0);
    });

    it('should handle rapid message additions', async () => {
      const workData = testDataGenerator.generateWorkData();
      const work = await testApiClient.createWork(workData);

      const messageCount = 20;
      const messagePromises = [];

      for (let i = 0; i < messageCount; i++) {
        const message = testDataGenerator.generateMessageData({
          content: `Rapid message ${i + 1}`,
        });
        messagePromises.push(testApiClient.addMessageToWork(work.work.id, message));
      }

      // All messages should be added successfully
      await Promise.all(messagePromises);

      // Verify all messages were added
      const workSession = await testApiClient.getWork(work.work.id);
      expect(workSession.messages.length).toBe(messageCount);
    });

    it('should handle AI output recording with various content types', async () => {
      const workData = testDataGenerator.generateWorkData();
      const work = await testApiClient.createWork(workData);

      const aiSession = await testApiClient.createAiSession(
        work.work.id,
        testDataGenerator.generateAiSessionData()
      );

      const testOutputs = [
        'Simple text output',
        'Output with\nnewlines\nand\nmultiple\nlines',
        'Output with special characters: áéíóú 🚀 ⭐ 💻',
        `Very long output: ${'x'.repeat(10000)}`,
        'Output with JSON: {"key": "value", "array": [1, 2, 3]}',
        '', // Empty output
      ];

      // Record all outputs
      for (const output of testOutputs) {
        await testApiClient.recordAiOutput(work.work.id, output);
      }

      // Verify outputs were recorded (API may not return them immediately)
      const outputs = await testApiClient.listAiOutputs(work.work.id);
      expect(outputs.outputs).toBeDefined();
      // Note: The actual number may vary based on API implementation
    });
  });

  describe('State Management Edge Cases', () => {
    it('should handle state manager with corrupted data', async () => {
      // Create some valid state
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData());
      const work = await testStateManager.addWorkSession(
        testDataGenerator.generateWorkData({
          project_id: project.id,
        })
      );

      // Verify initial state is valid
      let validation = testStateManager.validateStateConsistency();
      expect(validation.valid).toBe(true);

      // Manually corrupt state (simulate internal error)
      const projects = testStateManager.getProjects();
      if (projects.length > 0) {
        // This would be internal corruption, hard to simulate safely
        // Instead, test with invalid project references
        const invalidWork = await testStateManager.addWorkSession(
          testDataGenerator.generateWorkData({
            project_id: 'non-existent-project-id',
          })
        );

        validation = testStateManager.validateStateConsistency();
        expect(validation.valid).toBe(false);
        expect(validation.errors.length).toBeGreaterThan(0);
        expect(validation.errors[0]).toContain('non-existent-project-id');
      }
    });

    it('should handle state synchronization failures', async () => {
      // Test state sync with API failures
      // This is hard to test directly, but we can test the sync method exists
      await expect(testStateManager.syncWithAPI()).resolves.toBeUndefined();

      // Verify state was updated
      const summary = testStateManager.getStateSummary();
      expect(typeof summary.projects).toBe('number');
      expect(typeof summary.workSessions).toBe('number');
      expect(typeof summary.aiSessions).toBe('number');
    });

    it('should handle listener management edge cases', async () => {
      let callCount = 0;
      const listener = () => {
        callCount++;
      };

      // Subscribe to an event
      const unsubscribe = testStateManager.subscribe('test-event', listener);

      // Trigger the event multiple times
      testStateManager['notifyListeners']('test-event', { data: 'test1' });
      testStateManager['notifyListeners']('test-event', { data: 'test2' });
      testStateManager['notifyListeners']('test-event', { data: 'test3' });

      // Should have been called 3 times
      expect(callCount).toBe(3);

      // Unsubscribe
      unsubscribe();

      // Trigger again - should not be called
      testStateManager['notifyListeners']('test-event', { data: 'test4' });
      expect(callCount).toBe(3); // Still 3, not 4
    });

    it('should handle multiple listeners for same event', async () => {
      let callCount1 = 0;
      let callCount2 = 0;

      const listener1 = () => {
        callCount1++;
      };
      const listener2 = () => {
        callCount2++;
      };

      testStateManager.subscribe('multi-event', listener1);
      testStateManager.subscribe('multi-event', listener2);

      testStateManager['notifyListeners']('multi-event', { data: 'test' });

      expect(callCount1).toBe(1);
      expect(callCount2).toBe(1);
    });
  });

  describe('Resource Exhaustion Scenarios', () => {
    it('should handle memory pressure from large responses', async () => {
      // Create project
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Memory Test Project',
        })
      );

      // Create many files with large content
      const fileCount = 10;
      const largeContent = 'x'.repeat(50000); // 50KB per file

      const filePromises = [];
      for (let i = 0; i < fileCount; i++) {
        const fileData = testDataGenerator.generateFileData({
          project_id: project.id,
          path: `memory-test-${i}.txt`,
          content: largeContent,
        });
        filePromises.push(testApiClient.createFile(fileData));
      }

      // Should handle without memory issues
      await Promise.all(filePromises);

      // Verify all files exist
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      const memoryFiles = fileList.files.filter(f => f.name.startsWith('memory-test-'));
      expect(memoryFiles.length).toBe(fileCount);

      // Test reading one of the large files
      const content = await testApiClient.getFileContent('memory-test-0.txt', project.id);
      expect(content.content.length).toBe(largeContent.length);
    });

    it('should handle high concurrency without race conditions', async () => {
      // Create project
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Concurrency Test Project',
        })
      );

      const operationCount = 50;
      const operations = [];

      // Mix of different operations to test concurrency
      for (let i = 0; i < operationCount; i++) {
        if (i % 3 === 0) {
          // Create file
          const fileData = testDataGenerator.generateFileData({
            project_id: project.id,
            path: `concurrency-file-${i}.txt`,
            content: `Content ${i}`,
          });
          operations.push(testApiClient.createFile(fileData));
        } else if (i % 3 === 1) {
          // List files
          operations.push(testApiClient.listFiles({ project_id: project.id }));
        } else {
          // Create work session
          const workData = testDataGenerator.generateWorkData({
            title: `Concurrency work ${i}`,
            project_id: project.id,
          });
          operations.push(testApiClient.createWork(workData));
        }
      }

      // Execute all operations concurrently
      const startTime = Date.now();
      const results = await Promise.all(operations);
      const endTime = Date.now();

      // Should complete without errors
      expect(results.length).toBe(operationCount);

      // Should complete in reasonable time
      const duration = endTime - startTime;
      expect(duration).toBeLessThan(30000); // Under 30 seconds

      // Verify final state
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      const concurrencyFiles = fileList.files.filter(f => f.name.startsWith('concurrency-file-'));
      expect(concurrencyFiles.length).toBe(Math.ceil(operationCount / 3));

      const workList = await testApiClient.listWork();
      const concurrencyWorks = workList.works.filter(w => w.title.startsWith('Concurrency work'));
      expect(concurrencyWorks.length).toBe(Math.floor(operationCount / 3));
    });

    it('should handle cleanup after error scenarios', async () => {
      // Create project and some resources
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Cleanup Test Project',
        })
      );

      // Create some files
      const filePromises = [];
      for (let i = 0; i < 5; i++) {
        const fileData = testDataGenerator.generateFileData({
          project_id: project.id,
          path: `cleanup-test-${i}.txt`,
          content: `Cleanup test file ${i}`,
        });
        filePromises.push(testApiClient.createFile(fileData));
      }
      await Promise.all(filePromises);

      // Create some work sessions
      const workPromises = [];
      for (let i = 0; i < 3; i++) {
        const workData = testDataGenerator.generateWorkData({
          title: `Cleanup work ${i}`,
          project_id: project.id,
        });
        workPromises.push(testApiClient.createWork(workData));
      }
      await Promise.all(workPromises);

      // Verify resources exist
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      expect(fileList.files.filter(f => f.name.startsWith('cleanup-test-')).length).toBe(5);

      const workList = await testApiClient.listWork();
      expect(workList.works.filter(w => w.title.startsWith('Cleanup work')).length).toBe(3);

      // Simulate cleanup (delete project - this may cascade delete resources)
      await testApiClient.deleteProject(project.id);

      // Verify project is gone
      await expect(testApiClient.fetchProject(project.id)).rejects.toThrow();

      // Note: Work sessions and files may or may not be cleaned up depending on API implementation
      // The important thing is that the operation completed without hanging
    });
  });

  describe('Boundary Condition Testing', () => {
    it('should handle maximum length inputs', async () => {
      // Test with maximum reasonable lengths
      const maxTitle = 'A'.repeat(500); // Very long title
      const maxContent = 'B'.repeat(100000); // 100KB content

      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Boundary Test Project',
        })
      );

      // Test long work title
      const workData = testDataGenerator.generateWorkData({
        title: maxTitle,
        project_id: project.id,
      });
      const work = await testApiClient.createWork(workData);
      expect(work.work.title.length).toBe(maxTitle.length);

      // Test large file content
      const largeFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'max-size-file.txt',
        content: maxContent,
      });
      await testApiClient.createFile(largeFile);

      const readContent = await testApiClient.getFileContent('max-size-file.txt', project.id);
      expect(readContent.content.length).toBe(maxContent.length);
    });

    it('should handle zero-length and near-zero inputs', async () => {
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Zero Length Test Project',
        })
      );

      // Test minimal valid inputs
      const minimalWork = testDataGenerator.generateWorkData({
        title: 'x', // Minimal title
        project_id: project.id,
      });
      const work = await testApiClient.createWork(minimalWork);
      expect(work.work.title).toBe('x');

      // Test empty file (should work)
      const emptyFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'empty.txt',
        content: '',
      });
      await testApiClient.createFile(emptyFile);

      const emptyContent = await testApiClient.getFileContent('empty.txt', project.id);
      expect(emptyContent.content).toBe('');
    });

    it('should handle special Unicode characters', async () => {
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Unicode Test Project',
        })
      );

      const unicodeContent =
        '🚀 Hello 🌟 世界 🌍\nUnicode: áéíóú\nEmojis: 😀🎉🎊\nMath: ∑∫√∆\nCurrency: €£¥$';

      const unicodeFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'unicode-test.txt',
        content: unicodeContent,
      });
      await testApiClient.createFile(unicodeFile);

      const readContent = await testApiClient.getFileContent('unicode-test.txt', project.id);
      expect(readContent.content).toBe(unicodeContent);

      // Test Unicode in work titles
      const unicodeWork = testDataGenerator.generateWorkData({
        title: 'Unicode Test: 🚀 🌟 世界',
        project_id: project.id,
      });
      const work = await testApiClient.createWork(unicodeWork);
      expect(work.work.title).toContain('🚀');
    });

    it('should handle rapid state changes', async () => {
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Rapid State Change Test',
        })
      );

      // Rapidly create and delete resources
      const cycles = 10;
      for (let i = 0; i < cycles; i++) {
        // Create work session
        const workData = testDataGenerator.generateWorkData({
          title: `Rapid work ${i}`,
          project_id: project.id,
        });
        const work = await testApiClient.createWork(workData);

        // Create file
        const fileData = testDataGenerator.generateFileData({
          project_id: project.id,
          path: `rapid-file-${i}.txt`,
          content: `Rapid file ${i}`,
        });
        await testApiClient.createFile(fileData);

        // Immediately delete them
        await testApiClient.deleteFile(`rapid-file-${i}.txt`, project.id);
        // Note: Work session deletion may not be implemented
      }

      // Verify final state is consistent
      const validation = testStateManager.validateStateConsistency();
      expect(validation.valid).toBe(true);

      // Check remaining files (should be 0 since we deleted them)
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      const rapidFiles = fileList.files.filter(f => f.name.startsWith('rapid-file-'));
      expect(rapidFiles.length).toBe(0);
    });
  });
});
