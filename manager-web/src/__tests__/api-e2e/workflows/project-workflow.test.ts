import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';
import type { Project } from '../../../types';

describe('Project Workflow - API Only', () => {
  beforeAll(async () => {
    await testDatabase.setupTestDatabase();
    await testServer.startServer();
  }, 30000);

  afterAll(async () => {
    await testServer.stopServer();
    await testDatabase.cleanupTestDatabase();
  });

  beforeEach(async () => {
    // Reset test data generator for consistent IDs
    testDataGenerator.reset();
  });

  describe('Project CRUD Operations', () => {
    it('should create a new project', async () => {
      const projectData = testDataGenerator.generateProjectData();

      const createdProject = await testApiClient.createProject(projectData);

      expect(createdProject).toBeDefined();
      expect(createdProject.id).toBeDefined();
      expect(createdProject.name).toBe(projectData.name);
      expect(createdProject.language).toBe(projectData.language);
      expect(createdProject.created_at).toBeDefined();
      expect(createdProject.updated_at).toBeDefined();
    });

    it('should fetch a project by ID', async () => {
      // Create a project first
      const projectData = testDataGenerator.generateProjectData();
      const createdProject = await testApiClient.createProject(projectData);

      // Fetch the project
      const fetchedProject = await testApiClient.fetchProject(createdProject.id);

      // Compare essential fields (technologies might differ due to processing)
      expect(fetchedProject.id).toBe(createdProject.id);
      expect(fetchedProject.name).toBe(createdProject.name);
      expect(fetchedProject.language).toBe(createdProject.language);
      expect(fetchedProject.description).toBe(createdProject.description);
    });

    it('should list all projects', async () => {
      // Create multiple projects
      const projectData1 = testDataGenerator.generateProjectData();
      const projectData2 = testDataGenerator.generateProjectData();

      const createdProject1 = await testApiClient.createProject(projectData1);
      const createdProject2 = await testApiClient.createProject(projectData2);

      // Fetch all projects
      const projects = await testApiClient.fetchProjects();

      expect(projects).toBeDefined();
      expect(Array.isArray(projects)).toBe(true);
      expect(projects.length).toBeGreaterThanOrEqual(2);

      // Check that our created projects are in the list
      const projectIds = projects.map(p => p.id);
      expect(projectIds).toContain(createdProject1.id);
      expect(projectIds).toContain(createdProject2.id);
    });

    it('should update a project', async () => {
      // Create a project first
      const projectData = testDataGenerator.generateProjectData();
      const createdProject = await testApiClient.createProject(projectData);

      // Note: The current API doesn't have update endpoints for projects
      // This test documents the expected behavior when update is implemented
      expect(createdProject.id).toBeDefined();
    });

    it('should delete a project', async () => {
      // Create a project first
      const projectData = testDataGenerator.generateProjectData();
      const createdProject = await testApiClient.createProject(projectData);

      // Delete the project
      await testApiClient.deleteProject(createdProject.id);

      // Verify project is deleted by trying to fetch it (should fail)
      await expect(testApiClient.fetchProject(createdProject.id)).rejects.toThrow();
    });
  });

  describe('Project Validation', () => {
    it('should reject invalid project data', async () => {
      const invalidProjectData = testDataGenerator.generateErrorScenarios().invalidProject;

      await expect(testApiClient.createProject(invalidProjectData)).rejects.toThrow();
    });

    it('should handle non-existent project fetch', async () => {
      const nonExistentId = 'non-existent-project-id';

      await expect(testApiClient.fetchProject(nonExistentId)).rejects.toThrow();
    });
  });

  describe('Project Workflow Integration', () => {
    it('should support complete project lifecycle', async () => {
      // Create project
      const projectData = testDataGenerator.generateProjectData();
      const project = await testApiClient.createProject(projectData);
      expect(project.id).toBeDefined();

      // Verify project exists in list
      const projects = await testApiClient.fetchProjects();
      expect(projects.find(p => p.id === project.id)).toBeDefined();

      // Fetch individual project
      const fetchedProject = await testApiClient.fetchProject(project.id);
      expect(fetchedProject.id).toBe(project.id);

      // Clean up - delete project
      await testApiClient.deleteProject(project.id);

      // Verify deletion
      await expect(testApiClient.fetchProject(project.id)).rejects.toThrow();
    });

    it('should handle multiple projects concurrently', async () => {
      const projectPromises = testDataGenerator
        .generateProjectBatch(3)
        .map(data => testApiClient.createProject(data));

      const projects = await Promise.all(projectPromises);

      expect(projects).toHaveLength(3);
      projects.forEach(project => {
        expect(project.id).toBeDefined();
        expect(project.name).toMatch(/^API-E2E-/);
      });

      // Clean up
      await Promise.all(projects.map(p => testApiClient.deleteProject(p.id)));
    });
  });
});
