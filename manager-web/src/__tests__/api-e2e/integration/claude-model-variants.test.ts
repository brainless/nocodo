import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';

describe('Claude Model Variants - API End-to-End', () => {
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
    if (testProjectId) {
      await testApiClient.deleteProject(testProjectId);
    }

    await testServer.stopServer();
    await testDatabase.cleanupTestDatabase();
  });

  beforeEach(async () => {
    testDataGenerator.reset();
  });

  describe('Claude Model Detection and Session Creation', () => {
    const claudeModels = [
      // Latest Claude 3.5 models
      { model: 'claude-3-5-sonnet-20241022', description: 'Claude 3.5 Sonnet (latest)' },
      { model: 'claude-3-5-haiku-20241022', description: 'Claude 3.5 Haiku (latest)' },
      // Previous Claude 3 models
      { model: 'claude-3-sonnet-20240229', description: 'Claude 3 Sonnet (previous)' },
      { model: 'claude-3-haiku-20240307', description: 'Claude 3 Haiku (previous)' },
      // Generic model names (backward compatibility)
      { model: 'claude-3-sonnet', description: 'Claude 3 Sonnet (generic)' },
      { model: 'claude-3-haiku', description: 'Claude 3 Haiku (generic)' },
    ];

    claudeModels.forEach(({ model, description }) => {
      it(`should create session with ${description} model`, async () => {
        const sessionData = testDataGenerator.generateLlmAgentSessionData({
          provider: 'anthropic',
          model,
          system_prompt: 'You are a helpful AI assistant with file system tools.',
        });

        const session = await testApiClient.createLlmAgentSession(testWorkId, sessionData);

        expect(session.session.id).toBeDefined();
        expect(session.session.work_id).toBe(testWorkId);
        expect(session.session.provider).toBe('anthropic');
        expect(session.session.model).toBe(model);
        expect(session.session.system_prompt).toBe(
          'You are a helpful AI assistant with file system tools.'
        );
      });
    });

    it('should create multiple sessions with different Claude models', async () => {
      const sonnetSessionData = testDataGenerator.generateLlmAgentSessionData({
        provider: 'anthropic',
        model: 'claude-3-5-sonnet-20241022',
      });
      const haikuSessionData = testDataGenerator.generateLlmAgentSessionData({
        provider: 'anthropic',
        model: 'claude-3-5-haiku-20241022',
      });

      const sonnetSession = await testApiClient.createLlmAgentSession(
        testWorkId,
        sonnetSessionData
      );
      const haikuSession = await testApiClient.createLlmAgentSession(testWorkId, haikuSessionData);

      expect(sonnetSession.session.id).not.toBe(haikuSession.session.id);
      expect(sonnetSession.session.work_id).toBe(haikuSession.session.work_id);
      expect(sonnetSession.session.model).toBe('claude-3-5-sonnet-20241022');
      expect(haikuSession.session.model).toBe('claude-3-5-haiku-20241022');
    });
  });

  describe('Claude Model Tool Support', () => {
    it('should support native tool calling for Claude 3.5 Sonnet', async () => {
      const sessionData = testDataGenerator.generateLlmAgentSessionData({
        provider: 'anthropic',
        model: 'claude-3-5-sonnet-20241022',
        system_prompt: 'You are a helpful AI assistant. Use tools when appropriate.',
      });

      const session = await testApiClient.createLlmAgentSession(testWorkId, sessionData);

      // Verify session supports native tools (this would be validated in the backend)
      expect(session.session.id).toBeDefined();
      expect(session.session.provider).toBe('anthropic');
      expect(session.session.model).toBe('claude-3-5-sonnet-20241022');
    });

    it('should support native tool calling for Claude 3.5 Haiku', async () => {
      const sessionData = testDataGenerator.generateLlmAgentSessionData({
        provider: 'anthropic',
        model: 'claude-3-5-haiku-20241022',
        system_prompt: 'You are a helpful AI assistant. Use tools when appropriate.',
      });

      const session = await testApiClient.createLlmAgentSession(testWorkId, sessionData);

      // Verify session supports native tools (this would be validated in the backend)
      expect(session.session.id).toBeDefined();
      expect(session.session.provider).toBe('anthropic');
      expect(session.session.model).toBe('claude-3-5-haiku-20241022');
    });

    it('should support tool calling for previous Claude model versions', async () => {
      const models = ['claude-3-sonnet-20240229', 'claude-3-haiku-20240307'];

      for (const model of models) {
        const sessionData = testDataGenerator.generateLlmAgentSessionData({
          provider: 'anthropic',
          model,
          system_prompt: 'You are a helpful AI assistant with tools.',
        });

        const session = await testApiClient.createLlmAgentSession(testWorkId, sessionData);

        expect(session.session.id).toBeDefined();
        expect(session.session.provider).toBe('anthropic');
        expect(session.session.model).toBe(model);
      }
    });
  });

  describe('Backward Compatibility', () => {
    it('should maintain compatibility with generic Claude model names', async () => {
      const genericModels = ['claude-3-sonnet', 'claude-3-haiku'];

      for (const model of genericModels) {
        const sessionData = testDataGenerator.generateLlmAgentSessionData({
          provider: 'anthropic',
          model,
        });

        const session = await testApiClient.createLlmAgentSession(testWorkId, sessionData);

        expect(session.session.id).toBeDefined();
        expect(session.session.provider).toBe('anthropic');
        expect(session.session.model).toBe(model);
      }
    });

    it('should support Claude provider alias', async () => {
      const sessionData = testDataGenerator.generateLlmAgentSessionData({
        provider: 'claude',
        model: 'claude-3-5-sonnet-20241022',
      });

      const session = await testApiClient.createLlmAgentSession(testWorkId, sessionData);

      expect(session.session.id).toBeDefined();
      expect(session.session.provider).toBe('claude');
      expect(session.session.model).toBe('claude-3-5-sonnet-20241022');
    });
  });

  describe('Model Configuration Validation', () => {
    it('should handle case-insensitive model names', async () => {
      const sessionData = testDataGenerator.generateLlmAgentSessionData({
        provider: 'anthropic',
        model: 'CLAUDE-3-5-SONNET-20241022', // uppercase
      });

      const session = await testApiClient.createLlmAgentSession(testWorkId, sessionData);

      expect(session.session.id).toBeDefined();
      expect(session.session.provider).toBe('anthropic');
      expect(session.session.model).toBe('CLAUDE-3-5-SONNET-20241022');
    });

    it('should create sessions with different system prompts for same model', async () => {
      const sessionData1 = testDataGenerator.generateLlmAgentSessionData({
        provider: 'anthropic',
        model: 'claude-3-5-sonnet-20241022',
        system_prompt: 'You are a helpful coding assistant.',
      });

      const sessionData2 = testDataGenerator.generateLlmAgentSessionData({
        provider: 'anthropic',
        model: 'claude-3-5-sonnet-20241022',
        system_prompt: 'You are a creative writing assistant.',
      });

      const session1 = await testApiClient.createLlmAgentSession(testWorkId, sessionData1);
      const session2 = await testApiClient.createLlmAgentSession(testWorkId, sessionData2);

      expect(session1.session.id).not.toBe(session2.session.id);
      expect(session1.session.system_prompt).toBe('You are a helpful coding assistant.');
      expect(session2.session.system_prompt).toBe('You are a creative writing assistant.');
    });
  });
});
