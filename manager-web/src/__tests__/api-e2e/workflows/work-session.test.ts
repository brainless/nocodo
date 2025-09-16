import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';

describe('Work Session - API Only', () => {
  beforeAll(async () => {
    await testDatabase.setupTestDatabase();
    await testServer.startServer();
  }, 30000);

  afterAll(async () => {
    await testServer.stopServer();
    await testDatabase.cleanupTestDatabase();
  });

  beforeEach(async () => {
    testDataGenerator.reset();
  });

  describe('Work Session CRUD Operations', () => {
    it('should create a new work session', async () => {
      const workData = testDataGenerator.generateWorkData();

      const createdWork = await testApiClient.createWork(workData);

      expect(createdWork).toBeDefined();
      expect(createdWork.work).toBeDefined();
      expect(createdWork.work.id).toBeDefined();
      expect(createdWork.work.title).toBe(workData.title);
      expect(createdWork.work.tool_name).toBeNull(); // tool_name is not set during creation
      expect(createdWork.work.status).toBeDefined();
      expect(createdWork.work.created_at).toBeDefined();
    });

    it('should retrieve a work session by ID', async () => {
      // Create a work session first
      const workData = testDataGenerator.generateWorkData();
      const createdWork = await testApiClient.createWork(workData);

      // Retrieve the work session
      const retrievedWork = await testApiClient.getWork(createdWork.work.id);

      expect(retrievedWork).toBeDefined();
      expect(retrievedWork.work.id).toBe(createdWork.work.id);
      expect(retrievedWork.work.title).toBe(workData.title);
      expect(retrievedWork.messages).toBeDefined();
      expect(Array.isArray(retrievedWork.messages)).toBe(true);
    });

    it('should list all work sessions', async () => {
      // Create multiple work sessions
      const workData1 = testDataGenerator.generateWorkData();
      const workData2 = testDataGenerator.generateWorkData();

      const createdWork1 = await testApiClient.createWork(workData1);
      const createdWork2 = await testApiClient.createWork(workData2);

      // List all work sessions
      const workList = await testApiClient.listWork();

      expect(workList).toBeDefined();
      expect(workList.works).toBeDefined();
      expect(Array.isArray(workList.works)).toBe(true);
      expect(workList.works.length).toBeGreaterThanOrEqual(2);

      // Check that our created work sessions are in the list
      const workIds = workList.works.map(w => w.id);
      expect(workIds).toContain(createdWork1.work.id);
      expect(workIds).toContain(createdWork2.work.id);
    });
  });

  describe('Message Operations', () => {
    let testWorkId: string;

    beforeAll(async () => {
      // Create a test work session for message operations
      const workData = testDataGenerator.generateWorkData();
      const work = await testApiClient.createWork(workData);
      testWorkId = work.work.id;
    });

    it('should add a message to a work session', async () => {
      const messageData = testDataGenerator.generateMessageData({
        content: 'Test message for work session',
      });

      const addedMessage = await testApiClient.addMessageToWork(testWorkId, messageData);

      expect(addedMessage).toBeDefined();
      expect(addedMessage.id).toBeDefined();
      expect(addedMessage.content).toBe(messageData.content);
      expect(addedMessage.content_type).toBe(messageData.content_type);
      expect(addedMessage.author_type).toBe(messageData.author_type);
      expect(addedMessage.created_at).toBeDefined();
    });

    it('should retrieve work session with messages', async () => {
      // Add a message first
      const messageData = testDataGenerator.generateMessageData({
        content: 'Message for retrieval test',
      });
      await testApiClient.addMessageToWork(testWorkId, messageData);

      // Retrieve work session with messages
      const workWithMessages = await testApiClient.getWork(testWorkId);

      expect(workWithMessages).toBeDefined();
      expect(workWithMessages.work.id).toBe(testWorkId);
      expect(workWithMessages.messages).toBeDefined();
      expect(workWithMessages.messages.length).toBeGreaterThan(0);

      // Check the message content
      const lastMessage = workWithMessages.messages[workWithMessages.messages.length - 1];
      expect(lastMessage.content).toBe(messageData.content);
      expect(lastMessage.content_type).toBe(messageData.content_type);
    });
  });

  describe('AI Session Operations', () => {
    let testWorkId: string;

    beforeAll(async () => {
      // Create a test work session for AI session operations
      const workData = testDataGenerator.generateWorkData();
      const work = await testApiClient.createWork(workData);
      testWorkId = work.work.id;
    });

    it('should create an AI session for a work session', async () => {
      const aiSessionData = testDataGenerator.generateAiSessionData();

      const createdAiSession = await testApiClient.createAiSession(testWorkId, aiSessionData);

      expect(createdAiSession).toBeDefined();
      expect(createdAiSession.session).toBeDefined();
      expect(createdAiSession.session.id).toBeDefined();
      expect(createdAiSession.session.work_id).toBe(testWorkId);
      expect(createdAiSession.session.tool_name).toBe(aiSessionData.tool_name);
      expect(createdAiSession.session.status).toBeDefined();
    });

    it('should record AI output for a work session', async () => {
      const outputContent = 'Test AI output response';

      const result = await testApiClient.recordAiOutput(testWorkId, outputContent);

      expect(result).toBeDefined();
      expect(result.ok).toBe(true);
    });

    it('should list AI outputs for a work session', async () => {
      // Record some outputs first
      await testApiClient.recordAiOutput(testWorkId, 'First output');
      await testApiClient.recordAiOutput(testWorkId, 'Second output');

      // List the outputs
      const outputs = await testApiClient.listAiOutputs(testWorkId);

      expect(outputs).toBeDefined();
      expect(outputs.outputs).toBeDefined();
      expect(Array.isArray(outputs.outputs)).toBe(true);
      // Note: The outputs might not be immediately available or might be empty
      // depending on the implementation
    });

    it('should send AI input to a work session', async () => {
      const inputContent = 'Test input to AI session';

      const result = await testApiClient.sendAiInput(testWorkId, inputContent);

      expect(result).toBeDefined();
      expect(result.ok).toBe(true);
    });
  });

  describe('Work Session Validation', () => {
    it('should reject invalid work session data', async () => {
      const invalidWorkData = testDataGenerator.generateErrorScenarios().invalidWork;

      await expect(testApiClient.createWork(invalidWorkData)).rejects.toThrow();
    });

    it('should handle non-existent work session retrieval', async () => {
      const nonExistentId = 'non-existent-work-id';

      await expect(testApiClient.getWork(nonExistentId)).rejects.toThrow();
    });

    it('should handle adding message to non-existent work session', async () => {
      const nonExistentId = 'non-existent-work-id';
      const messageData = testDataGenerator.generateMessageData();

      await expect(testApiClient.addMessageToWork(nonExistentId, messageData)).rejects.toThrow();
    });

    it('should handle creating AI session for non-existent work session', async () => {
      const nonExistentId = 'non-existent-work-id';
      const aiSessionData = testDataGenerator.generateAiSessionData();

      await expect(testApiClient.createAiSession(nonExistentId, aiSessionData)).rejects.toThrow();
    });
  });

  describe('Complete Work Session Workflow', () => {
    it('should support complete work session lifecycle with AI integration', async () => {
      // 1. Create work session
      const workData = testDataGenerator.generateWorkData({
        title: 'Complete workflow test',
        tool_name: 'llm-agent',
      });
      const work = await testApiClient.createWork(workData);
      const workId = work.work.id;

      expect(workId).toBeDefined();

      // 2. Add initial message (user prompt)
      const userMessage = testDataGenerator.generateMessageData({
        content: 'Please analyze the codebase and provide a summary',
        author_type: 'user',
      });
      const addedMessage = await testApiClient.addMessageToWork(workId, userMessage);
      expect(addedMessage.id).toBeDefined();

      // 3. Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData({
        tool_name: 'llm-agent',
      });
      const aiSession = await testApiClient.createAiSession(workId, aiSessionData);
      expect(aiSession.session.id).toBeDefined();

      // 4. Simulate AI processing (record output)
      const aiOutput = 'Based on my analysis of the codebase...';
      await testApiClient.recordAiOutput(workId, aiOutput);

      // 5. Send additional input if needed
      const additionalInput = 'Can you provide more details about the main functions?';
      await testApiClient.sendAiInput(workId, additionalInput);

      // 6. Verify the complete session
      const completeWork = await testApiClient.getWork(workId);
      expect(completeWork.work.id).toBe(workId);
      expect(completeWork.messages.length).toBeGreaterThan(0);

      // 7. List outputs
      const outputs = await testApiClient.listAiOutputs(workId);
      expect(outputs.outputs).toBeDefined();
    });

    it('should handle multiple concurrent work sessions', async () => {
      const workPromises = testDataGenerator.generateProjectBatch(3).map((_, index) =>
        testDataGenerator.generateWorkData({
          title: `Concurrent work session ${index + 1}`,
        })
      ).map(workData => testApiClient.createWork(workData));

      const works = await Promise.all(workPromises);

      expect(works).toHaveLength(3);
      works.forEach(work => {
        expect(work.work.id).toBeDefined();
        expect(work.work.title).toMatch(/^Concurrent work session/);
      });

      // Add messages to each work session concurrently
      const messagePromises = works.map(work =>
        testApiClient.addMessageToWork(
          work.work.id,
          testDataGenerator.generateMessageData({
            content: `Message for ${work.work.title}`,
          })
        )
      );

      const messages = await Promise.all(messagePromises);
      expect(messages).toHaveLength(3);
      messages.forEach(message => {
        expect(message.id).toBeDefined();
      });
    });
  });
});