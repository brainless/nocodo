import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';
import { testStateManager } from '../utils/state-manager';

describe('Performance and Load Testing - API Only', () => {
  beforeAll(async () => {
    await testDatabase.setupTestDatabase();
    await testServer.startServer();
    await testStateManager.initialize();
  }, 120000); // Extended timeout for performance tests

  afterAll(async () => {
    testStateManager.clearState();
    await testServer.stopServer();
    await testDatabase.cleanupTestDatabase();
  });

  beforeEach(async () => {
    testDataGenerator.reset();
  });

  describe('API Response Time Benchmarks', () => {
    it('should measure project CRUD operation performance', async () => {
      const operations = 50;
      const timings: number[] = [];

      // Measure create operations
      for (let i = 0; i < operations; i++) {
        const startTime = performance.now();
        const projectData = testDataGenerator.generateProjectData({
          name: `Perf Project ${i}`,
        });
        await testApiClient.createProject(projectData);
        const endTime = performance.now();
        timings.push(endTime - startTime);
      }

      // Calculate statistics
      const avgTime = timings.reduce((a, b) => a + b, 0) / timings.length;
      const minTime = Math.min(...timings);
      const maxTime = Math.max(...timings);
      const p95Time = timings.sort((a, b) => a - b)[Math.floor(timings.length * 0.95)];

      console.log(`Project Create Performance:
        Operations: ${operations}
        Average: ${avgTime.toFixed(2)}ms
        Min: ${minTime.toFixed(2)}ms
        Max: ${maxTime.toFixed(2)}ms
        P95: ${p95Time.toFixed(2)}ms`);

      // Performance assertions
      expect(avgTime).toBeLessThan(500); // Average under 500ms
      expect(p95Time).toBeLessThan(1000); // 95th percentile under 1s
      expect(minTime).toBeGreaterThan(0); // Some time should be taken

      // Verify all projects were created
      const projects = await testApiClient.fetchProjects();
      const perfProjects = projects.filter(p => p.name.startsWith('Perf Project'));
      expect(perfProjects.length).toBe(operations);
    });

    it('should measure file operation performance', async () => {
      // Create test project
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'File Perf Test Project',
      }));

      const operations = 30;
      const fileSize = 10000; // 10KB files
      const content = 'x'.repeat(fileSize);
      const timings: number[] = [];

      // Measure file create operations
      for (let i = 0; i < operations; i++) {
        const startTime = performance.now();
        const fileData = testDataGenerator.generateFileData({
          project_id: project.id,
          path: `perf-file-${i.toString().padStart(3, '0')}.txt`,
          content,
        });
        await testApiClient.createFile(fileData);
        const endTime = performance.now();
        timings.push(endTime - startTime);
      }

      // Calculate statistics
      const avgTime = timings.reduce((a, b) => a + b, 0) / timings.length;
      const throughput = (operations * fileSize) / (avgTime / 1000) / 1024 / 1024; // MB/s

      console.log(`File Create Performance (${fileSize} bytes each):
        Operations: ${operations}
        Average: ${avgTime.toFixed(2)}ms
        Throughput: ${throughput.toFixed(2)} MB/s`);

      // Performance assertions
      expect(avgTime).toBeLessThan(1000); // Average under 1s
      expect(throughput).toBeGreaterThan(0.1); // At least 0.1 MB/s

      // Verify all files were created
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      const perfFiles = fileList.files.filter(f => f.name.startsWith('perf-file-'));
      expect(perfFiles.length).toBe(operations);
    });

    it('should measure work session operation performance', async () => {
      const operations = 25;
      const timings: number[] = [];

      // Measure work session create operations
      for (let i = 0; i < operations; i++) {
        const startTime = performance.now();
        const workData = testDataGenerator.generateWorkData({
          title: `Perf Work Session ${i}`,
          tool_name: 'llm-agent',
        });
        await testApiClient.createWork(workData);
        const endTime = performance.now();
        timings.push(endTime - startTime);
      }

      const avgTime = timings.reduce((a, b) => a + b, 0) / timings.length;
      const minTime = Math.min(...timings);
      const maxTime = Math.max(...timings);

      console.log(`Work Session Create Performance:
        Operations: ${operations}
        Average: ${avgTime.toFixed(2)}ms
        Min: ${minTime.toFixed(2)}ms
        Max: ${maxTime.toFixed(2)}ms`);

      expect(avgTime).toBeLessThan(300); // Average under 300ms
      expect(maxTime).toBeLessThan(1000); // Max under 1s
    });
  });

  describe('Concurrent Load Testing', () => {
    it('should handle high concurrent project operations', async () => {
      const concurrentOperations = 20;
      const operationPromises = [];

      const startTime = performance.now();

      // Launch concurrent project creation operations
      for (let i = 0; i < concurrentOperations; i++) {
        const projectData = testDataGenerator.generateProjectData({
          name: `Concurrent Project ${i}`,
        });
        operationPromises.push(testApiClient.createProject(projectData));
      }

      // Wait for all operations to complete
      const results = await Promise.all(operationPromises);
      const endTime = performance.now();
      const totalTime = endTime - startTime;

      // Calculate metrics
      const avgTime = totalTime / concurrentOperations;
      const opsPerSecond = (concurrentOperations / totalTime) * 1000;

      console.log(`Concurrent Project Creation:
        Operations: ${concurrentOperations}
        Total Time: ${totalTime.toFixed(2)}ms
        Average per Operation: ${avgTime.toFixed(2)}ms
        Operations/second: ${opsPerSecond.toFixed(2)}`);

      // Verify all operations succeeded
      expect(results.length).toBe(concurrentOperations);
      results.forEach(result => {
        expect(result.id).toBeDefined();
        expect(result.name).toMatch(/^Concurrent Project/);
      });

      // Performance assertions
      expect(totalTime).toBeLessThan(10000); // Complete within 10 seconds
      expect(avgTime).toBeLessThan(1000); // Average under 1s per operation
    });

    it('should handle mixed concurrent operations', async () => {
      // Create base project
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Mixed Load Test Project',
      }));

      const operationCount = 50;
      const operations = [];

      // Mix different types of operations
      for (let i = 0; i < operationCount; i++) {
        if (i % 5 === 0) {
          // Create work session
          const workData = testDataGenerator.generateWorkData({
            title: `Mixed work ${i}`,
            project_id: project.id,
          });
          operations.push(testApiClient.createWork(workData));
        } else if (i % 5 === 1) {
          // Create file
          const fileData = testDataGenerator.generateFileData({
            project_id: project.id,
            path: `mixed-file-${i}.txt`,
            content: `Mixed file content ${i}`,
          });
          operations.push(testApiClient.createFile(fileData));
        } else if (i % 5 === 2) {
          // List files
          operations.push(testApiClient.listFiles({ project_id: project.id }));
        } else if (i % 5 === 3) {
          // Fetch project
          operations.push(testApiClient.fetchProject(project.id));
        } else {
          // List work sessions
          operations.push(testApiClient.listWork());
        }
      }

      const startTime = performance.now();
      const results = await Promise.all(operations);
      const endTime = performance.now();
      const totalTime = endTime - startTime;

      const avgTime = totalTime / operationCount;
      const opsPerSecond = (operationCount / totalTime) * 1000;

      console.log(`Mixed Concurrent Operations:
        Operations: ${operationCount}
        Total Time: ${totalTime.toFixed(2)}ms
        Average per Operation: ${avgTime.toFixed(2)}ms
        Operations/second: ${opsPerSecond.toFixed(2)}`);

      // Verify operations completed
      expect(results.length).toBe(operationCount);

      // Performance assertions
      expect(totalTime).toBeLessThan(15000); // Complete within 15 seconds
      expect(avgTime).toBeLessThan(500); // Average under 500ms per operation
    });

    it('should handle sustained load over time', async () => {
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Sustained Load Test Project',
      }));

      const duration = 10000; // 10 seconds
      const startTime = performance.now();
      let operationCount = 0;
      const timings: number[] = [];

      // Perform operations for the specified duration
      while (performance.now() - startTime < duration) {
        const opStartTime = performance.now();

        // Alternate between different operations
        if (operationCount % 3 === 0) {
          const fileData = testDataGenerator.generateFileData({
            project_id: project.id,
            path: `sustained-file-${operationCount}.txt`,
            content: `Sustained load test ${operationCount}`,
          });
          await testApiClient.createFile(fileData);
        } else if (operationCount % 3 === 1) {
          await testApiClient.listFiles({ project_id: project.id });
        } else {
          const workData = testDataGenerator.generateWorkData({
            title: `Sustained work ${operationCount}`,
            project_id: project.id,
          });
          await testApiClient.createWork(workData);
        }

        const opEndTime = performance.now();
        timings.push(opEndTime - opStartTime);
        operationCount++;
      }

      const endTime = performance.now();
      const actualDuration = endTime - startTime;

      // Calculate sustained performance metrics
      const avgResponseTime = timings.reduce((a, b) => a + b, 0) / timings.length;
      const opsPerSecond = (operationCount / actualDuration) * 1000;
      const totalThroughput = (operationCount / actualDuration) * 1000;

      console.log(`Sustained Load Performance:
        Duration: ${actualDuration.toFixed(2)}ms
        Total Operations: ${operationCount}
        Average Response Time: ${avgResponseTime.toFixed(2)}ms
        Operations/second: ${opsPerSecond.toFixed(2)}
        Total Throughput: ${totalThroughput.toFixed(2)} ops`);

      // Performance assertions for sustained load
      expect(operationCount).toBeGreaterThan(10); // At least 10 operations
      expect(avgResponseTime).toBeLessThan(1000); // Average under 1s
      expect(opsPerSecond).toBeGreaterThan(0.5); // At least 0.5 ops/sec sustained
    });
  });

  describe('Memory and Resource Usage', () => {
    it('should handle large dataset operations without memory issues', async () => {
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Memory Test Project',
      }));

      const fileCount = 100;
      const avgFileSize = 5000; // 5KB average
      const operations = [];

      // Create many files
      for (let i = 0; i < fileCount; i++) {
        const fileData = testDataGenerator.generateFileData({
          project_id: project.id,
          path: `memory-file-${i.toString().padStart(3, '0')}.txt`,
          content: `Memory test content ${i}\n${'x'.repeat(avgFileSize - 50)}`,
        });
        operations.push(testApiClient.createFile(fileData));
      }

      const startTime = performance.now();
      await Promise.all(operations);
      const endTime = performance.now();
      const duration = endTime - startTime;

      // Calculate memory usage metrics
      const totalDataSize = fileCount * avgFileSize;
      const throughput = totalDataSize / (duration / 1000) / 1024 / 1024; // MB/s

      console.log(`Memory Usage Test:
        Files Created: ${fileCount}
        Average File Size: ${avgFileSize} bytes
        Total Data: ${(totalDataSize / 1024 / 1024).toFixed(2)} MB
        Duration: ${duration.toFixed(2)}ms
        Throughput: ${throughput.toFixed(2)} MB/s`);

      // Verify all files exist
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      const memoryFiles = fileList.files.filter(f => f.name.startsWith('memory-file-'));
      expect(memoryFiles.length).toBe(fileCount);

      // Performance assertions
      expect(duration).toBeLessThan(30000); // Complete within 30 seconds
      expect(throughput).toBeGreaterThan(0.5); // At least 0.5 MB/s
    });

    it('should handle state manager memory usage under load', async () => {
      const initialState = testStateManager.getStateSummary();

      // Create many projects and work sessions
      const batchSize = 20;
      const batches = 3;

      for (let batch = 0; batch < batches; batch++) {
        const projectPromises = [];
        for (let i = 0; i < batchSize; i++) {
          const projectData = testDataGenerator.generateProjectData({
            name: `State Memory Project ${batch}-${i}`,
          });
          projectPromises.push(testStateManager.addProject(projectData));
        }

        const projects = await Promise.all(projectPromises);

        // Create work sessions for each project
        const workPromises = [];
        for (const project of projects) {
          for (let i = 0; i < 2; i++) {
            const workData = testDataGenerator.generateWorkData({
              title: `State Memory Work ${batch}-${project.id}-${i}`,
              project_id: project.id,
            });
            workPromises.push(testStateManager.addWorkSession(workData));
          }
        }

        await Promise.all(workPromises);

        // Validate state consistency after each batch
        const validation = testStateManager.validateStateConsistency();
        expect(validation.valid).toBe(true);
      }

      const finalState = testStateManager.getStateSummary();
      const stateGrowth = {
        projects: finalState.projects - initialState.projects,
        workSessions: finalState.workSessions - initialState.workSessions,
        aiSessions: finalState.aiSessions - initialState.aiSessions,
      };

      console.log(`State Manager Memory Usage:
        Initial State: ${JSON.stringify(initialState)}
        Final State: ${JSON.stringify(finalState)}
        Growth: ${JSON.stringify(stateGrowth)}`);

      // Verify expected growth
      expect(stateGrowth.projects).toBe(batchSize * batches);
      expect(stateGrowth.workSessions).toBe(batchSize * batches * 2);

      // Verify state manager can handle the load
      expect(finalState.projects).toBeLessThan(1000); // Reasonable upper bound
      expect(finalState.workSessions).toBeLessThan(2000); // Reasonable upper bound
    });
  });

  describe('Database Performance Under Load', () => {
    it('should measure database operation performance', async () => {
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'DB Perf Test Project',
      }));

      const operations = 100;
      const timings: { create: number[]; read: number[]; update: number[] } = {
        create: [],
        read: [],
        update: [],
      };

      // Create operations
      for (let i = 0; i < operations; i++) {
        const startTime = performance.now();
        const workData = testDataGenerator.generateWorkData({
          title: `DB Perf Work ${i}`,
          project_id: project.id,
        });
        await testApiClient.createWork(workData);
        const endTime = performance.now();
        timings.create.push(endTime - startTime);
      }

      // Read operations
      const workList = await testApiClient.listWork();
      const dbWorks = workList.works.filter(w => w.title.startsWith('DB Perf Work'));

      for (const work of dbWorks.slice(0, 50)) { // Test first 50 reads
        const startTime = performance.now();
        await testApiClient.getWork(work.id);
        const endTime = performance.now();
        timings.read.push(endTime - startTime);
      }

      // Calculate database performance metrics
      const createAvg = timings.create.reduce((a, b) => a + b, 0) / timings.create.length;
      const readAvg = timings.read.reduce((a, b) => a + b, 0) / timings.read.length;

      console.log(`Database Performance:
        Create Operations: ${timings.create.length}
        Create Average: ${createAvg.toFixed(2)}ms
        Read Operations: ${timings.read.length}
        Read Average: ${readAvg.toFixed(2)}ms`);

      // Database performance assertions
      expect(createAvg).toBeLessThan(200); // DB creates under 200ms
      expect(readAvg).toBeLessThan(100); // DB reads under 100ms
    });

    it('should handle database transaction integrity under concurrent load', async () => {
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'DB Transaction Test Project',
      }));

      const concurrentTransactions = 10;
      const operationsPerTransaction = 5;
      const transactionPromises = [];

      for (let t = 0; t < concurrentTransactions; t++) {
        const transactionPromise = (async () => {
          const transactionOperations = [];

          // Create multiple related operations in a "transaction"
          for (let i = 0; i < operationsPerTransaction; i++) {
            const workData = testDataGenerator.generateWorkData({
              title: `Transaction ${t} Work ${i}`,
              project_id: project.id,
            });
            transactionOperations.push(testApiClient.createWork(workData));
          }

          // Execute all operations for this "transaction"
          const results = await Promise.all(transactionOperations);

          // Verify all operations in this transaction succeeded
          expect(results.length).toBe(operationsPerTransaction);
          results.forEach(result => {
            expect(result.work.id).toBeDefined();
            expect(result.work.title).toContain(`Transaction ${t}`);
          });

          return results;
        })();

        transactionPromises.push(transactionPromise);
      }

      // Execute all concurrent transactions
      const startTime = performance.now();
      const transactionResults = await Promise.all(transactionPromises);
      const endTime = performance.now();
      const duration = endTime - startTime;

      const totalOperations = concurrentTransactions * operationsPerTransaction;
      const opsPerSecond = (totalOperations / duration) * 1000;

      console.log(`Database Transaction Integrity:
        Concurrent Transactions: ${concurrentTransactions}
        Operations per Transaction: ${operationsPerTransaction}
        Total Operations: ${totalOperations}
        Duration: ${duration.toFixed(2)}ms
        Operations/second: ${opsPerSecond.toFixed(2)}`);

      // Verify all transactions completed successfully
      expect(transactionResults.length).toBe(concurrentTransactions);
      transactionResults.forEach(results => {
        expect(results.length).toBe(operationsPerTransaction);
      });

      // Performance assertions
      expect(duration).toBeLessThan(10000); // Complete within 10 seconds
      expect(opsPerSecond).toBeGreaterThan(1); // At least 1 op/sec
    });
  });

  describe('System Resource Monitoring', () => {
    it('should monitor API response time degradation under load', async () => {
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Load Degradation Test Project',
      }));

      const testPhases = [
        { operations: 10, name: 'Light Load' },
        { operations: 50, name: 'Medium Load' },
        { operations: 100, name: 'Heavy Load' },
      ];

      const phaseResults: any[] = [];

      for (const phase of testPhases) {
        const timings: number[] = [];

        // Execute operations for this phase
        const phaseStartTime = performance.now();
        for (let i = 0; i < phase.operations; i++) {
          const opStartTime = performance.now();
          const workData = testDataGenerator.generateWorkData({
            title: `${phase.name} Work ${i}`,
            project_id: project.id,
          });
          await testApiClient.createWork(workData);
          const opEndTime = performance.now();
          timings.push(opEndTime - opStartTime);
        }
        const phaseEndTime = performance.now();

        // Calculate phase metrics
        const avgTime = timings.reduce((a, b) => a + b, 0) / timings.length;
        const p95Time = timings.sort((a, b) => a - b)[Math.floor(timings.length * 0.95)];
        const phaseDuration = phaseEndTime - phaseStartTime;

        phaseResults.push({
          phase: phase.name,
          operations: phase.operations,
          avgResponseTime: avgTime,
          p95ResponseTime: p95Time,
          totalDuration: phaseDuration,
          opsPerSecond: (phase.operations / phaseDuration) * 1000,
        });
      }

      // Analyze degradation
      const degradation = phaseResults.map((result, index) => ({
        ...result,
        degradation: index > 0 ? (result.avgResponseTime / phaseResults[0].avgResponseTime) : 1,
      }));

      console.log('Load Degradation Analysis:');
      degradation.forEach(result => {
        console.log(`  ${result.phase}:
    Operations: ${result.operations}
    Avg Response Time: ${result.avgResponseTime.toFixed(2)}ms
    P95 Response Time: ${result.p95ResponseTime.toFixed(2)}ms
    Degradation Factor: ${result.degradation.toFixed(2)}x
    Ops/sec: ${result.opsPerSecond.toFixed(2)}`);
      });

      // Performance degradation assertions
      // Light to medium load should not degrade more than 3x
      expect(degradation[1].degradation).toBeLessThan(3);
      // Medium to heavy load should not degrade more than 5x
      expect(degradation[2].degradation).toBeLessThan(5);

      // Overall system should maintain reasonable performance
      expect(degradation[2].avgResponseTime).toBeLessThan(2000); // Under 2s even under heavy load
    });

    it('should test system recovery after load spikes', async () => {
      const project = await testStateManager.addProject(testDataGenerator.generateProjectData({
        name: 'Recovery Test Project',
      }));

      // Phase 1: Normal load
      const normalOperations = 20;
      const normalTimings: number[] = [];

      for (let i = 0; i < normalOperations; i++) {
        const startTime = performance.now();
        const workData = testDataGenerator.generateWorkData({
          title: `Normal Work ${i}`,
          project_id: project.id,
        });
        await testApiClient.createWork(workData);
        const endTime = performance.now();
        normalTimings.push(endTime - startTime);
      }

      const normalAvg = normalTimings.reduce((a, b) => a + b, 0) / normalTimings.length;

      // Phase 2: Load spike
      const spikeOperations = 50;
      const spikeTimings: number[] = [];

      for (let i = 0; i < spikeOperations; i++) {
        const startTime = performance.now();
        const fileData = testDataGenerator.generateFileData({
          project_id: project.id,
          path: `spike-file-${i}.txt`,
          content: `Spike load test file ${i}\n${'x'.repeat(2000)}`,
        });
        await testApiClient.createFile(fileData);
        const endTime = performance.now();
        spikeTimings.push(endTime - startTime);
      }

      const spikeAvg = spikeTimings.reduce((a, b) => a + b, 0) / spikeTimings.length;

      // Phase 3: Recovery period
      const recoveryOperations = 15;
      const recoveryTimings: number[] = [];

      // Wait a bit for system to recover
      await new Promise(resolve => setTimeout(resolve, 1000));

      for (let i = 0; i < recoveryOperations; i++) {
        const startTime = performance.now();
        const workData = testDataGenerator.generateWorkData({
          title: `Recovery Work ${i}`,
          project_id: project.id,
        });
        await testApiClient.createWork(workData);
        const endTime = performance.now();
        recoveryTimings.push(endTime - startTime);
      }

      const recoveryAvg = recoveryTimings.reduce((a, b) => a + b, 0) / recoveryTimings.length;

      console.log(`System Recovery Test:
        Normal Load Avg: ${normalAvg.toFixed(2)}ms
        Spike Load Avg: ${spikeAvg.toFixed(2)}ms
        Recovery Load Avg: ${recoveryAvg.toFixed(2)}ms
        Spike Factor: ${(spikeAvg / normalAvg).toFixed(2)}x
        Recovery Factor: ${(recoveryAvg / normalAvg).toFixed(2)}x`);

      // Recovery assertions
      expect(spikeAvg / normalAvg).toBeLessThan(5); // Spike shouldn't be more than 5x slower
      expect(recoveryAvg / normalAvg).toBeLessThan(2); // Recovery should be close to normal (less than 2x)

      // System should recover to reasonable performance
      expect(recoveryAvg).toBeLessThan(1000); // Recovery operations under 1s avg
    });
  });
});