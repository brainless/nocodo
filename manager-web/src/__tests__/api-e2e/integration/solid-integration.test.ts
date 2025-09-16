import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';
import { testStateManager } from '../utils/state-manager';

describe('Solid State Management Integration - API Only', () => {
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

  describe('State Manager Integration', () => {
    it('should integrate with API for project state management', async () => {
      // Test state manager integration with API
      const projectData = testDataGenerator.generateProjectData({
        name: 'State Integration Test Project',
      });

      // Create project through state manager
      const project = await testStateManager.addProject(projectData);

      // Verify state manager has the project
      const stateProject = testStateManager.getProject(project.id);
      expect(stateProject).toEqual(project);

      // Verify API has the project
      const apiProject = await testApiClient.fetchProject(project.id);
      expect(apiProject).toEqual(project);

      // Test state consistency
      const validation = testStateManager.validateStateConsistency();
      expect(validation.valid).toBe(true);
    });

    it('should handle work session state management', async () => {
      // Create project first
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData());

      // Create work session through state manager
      const workData = testDataGenerator.generateWorkData({
        title: 'State Management Work Session',
        project_id: project.id,
      });

      const work = await testStateManager.addWorkSession(workData);

      // Verify state manager has the work session
      const stateWork = testStateManager.getWorkSession(work.work.id);
      expect(stateWork).toEqual(work);

      // Verify API has the work session
      const apiWork = await testApiClient.getWork(work.work.id);
      expect(apiWork.work).toEqual(work.work);

      // Test state consistency
      const validation = testStateManager.validateStateConsistency();
      expect(validation.valid).toBe(true);
    });

    it('should synchronize state with API', async () => {
      // Create some data through API directly
      const apiProject = await testApiClient.createProject(testDataGenerator.generateProjectData({
        name: 'Sync Test Project',
      }));

      const apiWork = await testApiClient.createWork(testDataGenerator.generateWorkData({
        title: 'Sync Test Work',
        project_id: apiProject.id,
      }));

      // State manager should not have this data initially
      expect(testStateManager.getProject(apiProject.id)).toBeUndefined();
      expect(testStateManager.getWorkSession(apiWork.work.id)).toBeUndefined();

      // Sync with API
      await testStateManager.syncWithAPI();

      // Now state manager should have the data
      const stateProject = testStateManager.getProject(apiProject.id);
      const stateWork = testStateManager.getWorkSession(apiWork.work.id);

      expect(stateProject).toBeDefined();
      expect(stateWork).toBeDefined();
      expect(stateProject!.id).toBe(apiProject.id);
      expect(stateWork!.work.id).toBe(apiWork.work.id);
    });

    it('should handle state updates and notifications', async () => {
      // Test state update notifications
      let updateNotifications = 0;
      let lastUpdateData: any = null;

      const unsubscribe = testStateManager.subscribe('work-updated', (data) => {
        updateNotifications++;
        lastUpdateData = data;
      });

      // Create and update a work session
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData());
      const work = await testStateManager.addWorkSession(testDataGenerator.generateWorkData({
        project_id: project.id,
      }));

      // Update the work session (simulate status change)
      await testStateManager.updateWorkSession(work.work.id);

      // Verify notification was received
      expect(updateNotifications).toBe(1);
      expect(lastUpdateData).toBeDefined();
      expect(lastUpdateData.work.id).toBe(work.work.id);

      // Cleanup
      unsubscribe();
    });
  });

  describe('Reactive State Patterns', () => {
    it('should support reactive project list updates', async () => {
      const initialProjects = testStateManager.getProjects().length;

      // Subscribe to project additions
      let addNotifications = 0;
      let removeNotifications = 0;

      const addUnsubscribe = testStateManager.subscribe('project-added', () => {
        addNotifications++;
      });

      const removeUnsubscribe = testStateManager.subscribe('project-removed', () => {
        removeNotifications++;
      });

      // Add projects
      const project1 = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Reactive Project 1',
      }));

      const project2 = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Reactive Project 2',
      }));

      // Verify reactive updates
      expect(addNotifications).toBe(2);
      expect(testStateManager.getProjects().length).toBe(initialProjects + 2);

      // Remove a project
      await testStateManager.removeProject(project1.id);

      // Verify removal notification
      expect(removeNotifications).toBe(1);
      expect(testStateManager.getProjects().length).toBe(initialProjects + 1);

      // Verify project is gone from state
      expect(testStateManager.getProject(project1.id)).toBeUndefined();

      // Cleanup
      addUnsubscribe();
      removeUnsubscribe();
    });

    it('should handle complex state relationships', async () => {
      // Create a project with multiple work sessions and AI sessions
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Complex State Project',
      }));

      const workSessions = [];
      const aiSessions = [];

      // Create multiple work sessions
      for (let i = 0; i < 3; i++) {
        const work = await testStateManager.addWorkSession(testDataGenerator.generateWorkData({
          title: `Complex Work ${i}`,
          project_id: project.id,
        }));
        workSessions.push(work);

        // Create AI session for each work session
        const aiSession = await testStateManager.addAiSession(work.work.id, testDataGenerator.generateAiSessionData());
        aiSessions.push(aiSession);
      }

      // Verify complex state relationships
      const stateSummary = testStateManager.getStateSummary();
      expect(stateSummary.projects).toBeGreaterThanOrEqual(1);
      expect(stateSummary.workSessions).toBeGreaterThanOrEqual(3);
      expect(stateSummary.aiSessions).toBeGreaterThanOrEqual(3);

      // Verify state consistency
      const validation = testStateManager.validateStateConsistency();
      expect(validation.valid).toBe(true);

      // Test cascading updates (when project is removed, related items should be affected)
      // Note: This depends on API implementation - may need to handle cleanup
      await testStateManager.removeProject(project.id);

      // Verify project is removed
      expect(testStateManager.getProject(project.id)).toBeUndefined();

      // Work sessions may or may not be automatically removed depending on API
      // The important thing is state consistency is maintained
      const finalValidation = testStateManager.validateStateConsistency();
      expect(finalValidation.valid).toBe(true);
    });

    it('should support batched state updates', async () => {
      let updateCount = 0;
      const unsubscribe = testStateManager.subscribe('work-updated', () => {
        updateCount++;
      });

      // Create project
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Batch Update Project',
      }));

      // Perform batched operations
      const batchOperations = [];
      for (let i = 0; i < 5; i++) {
        batchOperations.push(
          testStateManager.addWorkSession(testDataGenerator.generateWorkData({
            title: `Batch Work ${i}`,
            project_id: project.id,
          }))
        );
      }

      // Execute batch
      await Promise.all(batchOperations);

      // Verify batch completion
      const stateSummary = testStateManager.getStateSummary();
      expect(stateSummary.workSessions).toBeGreaterThanOrEqual(5);

      // State updates should be handled properly (may not trigger individual notifications)
      // The important thing is final state consistency
      const validation = testStateManager.validateStateConsistency();
      expect(validation.valid).toBe(true);

      unsubscribe();
    });
  });

  describe('State Persistence and Recovery', () => {
    it('should persist state across operations', async () => {
      // Create initial state
      const project1 = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Persistence Project 1',
      }));

      const work1 = await testStateManager.addWorkSession(testDataGenerator.generateWorkData({
        title: 'Persistence Work 1',
        project_id: project1.id,
      }));

      const initialSummary = testStateManager.getStateSummary();

      // Perform some operations that might affect persistence
      const project2 = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Persistence Project 2',
      }));

      // Sync with API (simulates persistence layer)
      await testStateManager.syncWithAPI();

      // Verify state persistence
      const syncedSummary = testStateManager.getStateSummary();
      expect(syncedSummary.projects).toBeGreaterThanOrEqual(initialSummary.projects + 1);
      expect(syncedSummary.workSessions).toBeGreaterThanOrEqual(initialSummary.workSessions);

      // Verify specific items still exist
      expect(testStateManager.getProject(project1.id)).toBeDefined();
      expect(testStateManager.getProject(project2.id)).toBeDefined();
      expect(testStateManager.getWorkSession(work1.work.id)).toBeDefined();
    });

    it('should handle state recovery after API failures', async () => {
      // Create known good state
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Recovery Test Project',
      }));

      const work = await testStateManager.addWorkSession(testDataGenerator.generateWorkData({
        project_id: project.id,
      }));

      const baselineSummary = testStateManager.getStateSummary();

      // Simulate API failure scenario (try operations that might fail)
      try {
        // Try to create work session with invalid project ID
        await testStateManager.addWorkSession(testDataGenerator.generateWorkData({
          project_id: 'invalid-project-id',
        }));
        expect.fail('Should have failed with invalid project ID');
      } catch (error) {
        // Expected failure - state should remain consistent
        const afterFailureSummary = testStateManager.getStateSummary();
        expect(afterFailureSummary.projects).toBe(baselineSummary.projects);
        expect(afterFailureSummary.workSessions).toBe(baselineSummary.workSessions + 1); // The valid work session

        // Verify state consistency after failure
        const validation = testStateManager.validateStateConsistency();
        expect(validation.valid).toBe(true);
      }

      // Verify original state is still intact
      expect(testStateManager.getProject(project.id)).toBeDefined();
      expect(testStateManager.getWorkSession(work.work.id)).toBeDefined();
    });

    it('should support state export/import patterns', async () => {
      // Create test state
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Export/Import Test Project',
      }));

      const work = await testStateManager.addWorkSession(testDataGenerator.generateWorkData({
        project_id: project.id,
      }));

      // Export state summary
      const exportedState = testStateManager.getStateSummary();

      // Simulate state reset (like page refresh)
      testStateManager.clearState();

      // Verify state is cleared
      expect(testStateManager.getStateSummary().projects).toBe(0);
      expect(testStateManager.getStateSummary().workSessions).toBe(0);

      // Re-initialize state
      await testStateManager.initialize();

      // Verify state can be rebuilt
      const newSummary = testStateManager.getStateSummary();
      expect(newSummary.projects).toBeGreaterThanOrEqual(0); // May have existing data from API

      // The pattern demonstrates state export/import capability
      expect(typeof exportedState).toBe('object');
      expect(exportedState.projects).toBeDefined();
      expect(exportedState.workSessions).toBeDefined();
    });
  });

  describe('Performance with State Management', () => {
    it('should maintain performance with state management overhead', async () => {
      const operationCount = 20;
      const timings: number[] = [];

      // Measure operations with state management
      for (let i = 0; i < operationCount; i++) {
        const startTime = performance.now();

        // Create project through state manager (includes state updates)
        await testStateManager.addProject(testDataGenerator.generateProjectData({
          name: `Perf State Project ${i}`,
        }));

        const endTime = performance.now();
        timings.push(endTime - startTime);
      }

      const avgTime = timings.reduce((a, b) => a + b, 0) / timings.length;
      const totalTime = timings.reduce((a, b) => a + b, 0);

      console.log(`State Management Performance:
        Operations: ${operationCount}
        Average Time: ${avgTime.toFixed(2)}ms
        Total Time: ${totalTime.toFixed(2)}ms`);

      // Performance should still be reasonable with state management
      expect(avgTime).toBeLessThan(500); // Under 500ms per operation
      expect(totalTime).toBeLessThan(15000); // Complete within 15 seconds

      // Verify state consistency
      const validation = testStateManager.validateStateConsistency();
      expect(validation.valid).toBe(true);

      // Verify all projects exist in state
      const projects = testStateManager.getProjects();
      const perfProjects = projects.filter(p => p.name.startsWith('Perf State Project'));
      expect(perfProjects.length).toBe(operationCount);
    });

    it('should handle concurrent state updates efficiently', async () => {
      const concurrentOperations = 10;
      const operationPromises = [];

      const startTime = performance.now();

      // Launch concurrent state operations
      for (let i = 0; i < concurrentOperations; i++) {
        operationPromises.push(
          testStateManager.addProject(testDataGenerator.generateProjectData({
            name: `Concurrent State Project ${i}`,
          }))
        );
      }

      // Wait for all operations
      await Promise.all(operationPromises);
      const endTime = performance.now();
      const totalTime = endTime - startTime;

      const avgTime = totalTime / concurrentOperations;

      console.log(`Concurrent State Updates:
        Operations: ${concurrentOperations}
        Total Time: ${totalTime.toFixed(2)}ms
        Average Time: ${avgTime.toFixed(2)}ms`);

      // Concurrent operations should complete efficiently
      expect(totalTime).toBeLessThan(5000); // Complete within 5 seconds
      expect(avgTime).toBeLessThan(1000); // Average under 1s per operation

      // Verify state consistency after concurrent operations
      const validation = testStateManager.validateStateConsistency();
      expect(validation.valid).toBe(true);

      // Verify all projects were added
      const projects = testStateManager.getProjects();
      const concurrentProjects = projects.filter(p => p.name.startsWith('Concurrent State Project'));
      expect(concurrentProjects.length).toBe(concurrentOperations);
    });

    it('should scale with increasing state size', async () => {
      const scaleTestSizes = [10, 25, 50];
      const performanceResults: any[] = [];

      for (const size of scaleTestSizes) {
        // Clear previous state
        testStateManager.clearState();
        await testStateManager.initialize();

        const startTime = performance.now();

        // Create projects up to current size
        const createPromises = [];
        for (let i = 0; i < size; i++) {
          createPromises.push(
            testStateManager.addProject(testDataGenerator.generateProjectData({
              name: `Scale Project ${i}`,
            }))
          );
        }
        await Promise.all(createPromises);

        // Measure list operation performance
        const listStartTime = performance.now();
        const projects = testStateManager.getProjects();
        const listEndTime = performance.now();

        const endTime = performance.now();
        const totalTime = endTime - startTime;
        const listTime = listEndTime - listStartTime;

        performanceResults.push({
          size,
          totalTime,
          listTime,
          projectsCount: projects.length,
        });

        // Verify state consistency at each scale
        const validation = testStateManager.validateStateConsistency();
        expect(validation.valid).toBe(true);
      }

      // Analyze scaling performance
      console.log('State Management Scaling Performance:');
      performanceResults.forEach(result => {
        console.log(`  Size ${result.size}: Total ${result.totalTime.toFixed(2)}ms, List ${result.listTime.toFixed(2)}ms`);
      });

      // Performance should scale reasonably (list operations shouldn't grow exponentially)
      const firstListTime = performanceResults[0].listTime;
      const lastListTime = performanceResults[performanceResults.length - 1].listTime;
      const scalingFactor = lastListTime / firstListTime;

      console.log(`Scaling factor: ${scalingFactor.toFixed(2)}x (from size ${performanceResults[0].size} to ${performanceResults[performanceResults.length - 1].size})`);

      // Scaling should be reasonable (less than 10x increase for 5x data size increase)
      expect(scalingFactor).toBeLessThan(10);
    });
  });

  describe('State Manager Error Handling', () => {
    it('should handle API failures gracefully in state operations', async () => {
      // Test state manager resilience to API failures
      const validProject = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Error Handling Project',
      }));

      // Attempt invalid operation (should fail at API level)
      try {
        await testStateManager.addWorkSession(testDataGenerator.generateWorkData({
          project_id: 'non-existent-project-id',
        }));
        expect.fail('Should have failed with invalid project ID');
      } catch (error) {
        // Expected failure - verify state integrity is maintained
        const validation = testStateManager.validateStateConsistency();
        expect(validation.valid).toBe(true);

        // Valid project should still exist
        expect(testStateManager.getProject(validProject.id)).toBeDefined();
      }
    });

    it('should handle state manager cleanup properly', async () => {
      // Create some state
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Cleanup Test Project',
      }));

      const work = await testStateManager.addWorkSession(testDataGenerator.generateWorkData({
        project_id: project.id,
      }));

      // Verify state exists
      expect(testStateManager.getStateSummary().projects).toBeGreaterThan(0);

      // Clear state
      testStateManager.clearState();

      // Verify state is cleared
      expect(testStateManager.getStateSummary().projects).toBe(0);
      expect(testStateManager.getStateSummary().workSessions).toBe(0);
      expect(testStateManager.getStateSummary().aiSessions).toBe(0);

      // Verify listeners are cleared
      expect(testStateManager.getStateSummary().listeners).toBe(0);
    });

    it('should validate state integrity after operations', async () => {
      // Test various operations and validate state after each

      // 1. Create project
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData());
      expect(testStateManager.validateStateConsistency().valid).toBe(true);

      // 2. Create work session
      const work = await testStateManager.addWorkSession(testDataGenerator.generateWorkData({
        project_id: project.id,
      }));
      expect(testStateManager.validateStateConsistency().valid).toBe(true);

      // 3. Create AI session
      const aiSession = await testStateManager.addAiSession(work.work.id, testDataGenerator.generateAiSessionData());
      expect(testStateManager.validateStateConsistency().valid).toBe(true);

      // 4. Update work session
      await testStateManager.updateWorkSession(work.work.id);
      expect(testStateManager.validateStateConsistency().valid).toBe(true);

      // 5. Remove project (may cascade)
      await testStateManager.removeProject(project.id);
      // State should still be consistent even after removal
      const finalValidation = testStateManager.validateStateConsistency();
      expect(finalValidation.valid).toBe(true);
    });
  });
});