import { describe, expect, it } from 'vitest';
import { testDataGenerator } from './test-data';
import { TestApiClient } from './api-client';

describe('API E2E Test Setup', () => {
  describe('Test Data Generator', () => {
    it('should generate valid project data', () => {
      const projectData = testDataGenerator.generateProjectData();

      expect(projectData).toBeDefined();
      expect(projectData.name).toMatch(/^API-E2E-/);
      expect(projectData.language).toBeDefined();
      expect(typeof projectData.name).toBe('string');
      expect(typeof projectData.language).toBe('string');
    });

    it('should generate valid work data', () => {
      const workData = testDataGenerator.generateWorkData();

      expect(workData).toBeDefined();
      expect(workData.title).toMatch(/^Test Work/);
      expect(workData.tool_name).toBe('llm-agent');
      expect(workData.project_id).toBeNull();
    });

    it('should generate valid message data', () => {
      const messageData = testDataGenerator.generateMessageData();

      expect(messageData).toBeDefined();
      expect(messageData.content).toBeDefined();
      expect(messageData.content_type).toBe('text');
      expect(messageData.author_type).toBe('user');
    });

    it('should generate valid AI session data', () => {
      const aiSessionData = testDataGenerator.generateAiSessionData();

      expect(aiSessionData).toBeDefined();
      expect(aiSessionData.tool_name).toBe('llm-agent');
      expect(aiSessionData.message_id).toBeDefined();
    });

    it('should generate valid file data', () => {
      const fileData = testDataGenerator.generateFileData();

      expect(fileData).toBeDefined();
      expect(fileData.path).toMatch(/^test-file/);
      expect(fileData.content).toBeDefined();
      expect(fileData.encoding).toBe('utf-8');
      expect(fileData.project_id).toBeDefined();
    });

    it('should generate consistent data with reset', () => {
      testDataGenerator.reset();

      const data1 = testDataGenerator.generateProjectData();
      const data2 = testDataGenerator.generateProjectData();

      expect(data1.name).not.toBe(data2.name); // Should be different due to timestamp
    });
  });

  describe('Test API Client', () => {
    it('should create client with default URL', () => {
      const client = new TestApiClient();

      expect(client).toBeDefined();
      // We can't test the actual HTTP calls without a server, but we can test instantiation
    });

    it('should create client with custom URL', () => {
      const customURL = 'http://localhost:9999';
      const client = new TestApiClient(customURL);

      expect(client).toBeDefined();
    });
  });

  describe('Error Scenarios Generator', () => {
    it('should generate invalid project data', () => {
      const invalidData = testDataGenerator.generateErrorScenarios().invalidProject;

      expect(invalidData.name).toBe(''); // Invalid empty name
    });

    it('should generate invalid work data', () => {
      const invalidData = testDataGenerator.generateErrorScenarios().invalidWork;

      expect(invalidData.title).toBe(''); // Invalid empty title
    });

    it('should generate invalid file data', () => {
      const invalidData = testDataGenerator.generateErrorScenarios().invalidFile;

      expect(invalidData.path).toBe(''); // Invalid empty path
    });
  });
});
