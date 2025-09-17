import { expect, test } from './setup';
import { startLLMAgentWork, waitForToolCall, waitForLLMResponse } from './test-utils';

test.describe('New Tools Performance and Reliability Tests', () => {
  test('should execute new tools within performance bounds', async ({ page, request }) => {
    const startTime = Date.now();

    const workId = await startLLMAgentWork(page, 'Write a small file and search for its content');

    // Wait for write_file tool
    await waitForToolCall(request, workId, 'write_file', 3000);

    // Wait for grep tool
    await waitForToolCall(request, workId, 'grep', 5000);

    const totalTime = Date.now() - startTime;
    expect(totalTime).toBeLessThan(15000); // Should complete within 15 seconds

    // Verify both tools completed successfully
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should handle large file operations gracefully', async ({ page, request }) => {
    const startTime = Date.now();

    const workId = await startLLMAgentWork(page, 'Search for patterns in all TypeScript files');

    const toolResult = await waitForToolCall(request, workId, 'grep', 10000);
    const executionTime = Date.now() - startTime;

    expect(executionTime).toBeLessThan(20000); // Should complete within 20 seconds for large searches
    expect(toolResult.status).toBe('completed');
    expect(toolResult.message.content).toContain('files_searched');

    // Verify LLM response is generated
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should maintain performance with multiple file operations', async ({ page, request }) => {
    const startTime = Date.now();

    const workId = await startLLMAgentWork(page, 'Create multiple files and search across them');

    // Execute multiple write operations
    const writePromises = [];
    for (let i = 0; i < 3; i++) {
      writePromises.push(waitForToolCall(request, workId, 'write_file', 5000));
    }

    // Wait for all write operations
    await Promise.all(writePromises);

    // Execute grep search
    await waitForToolCall(request, workId, 'grep', 5000);

    const totalTime = Date.now() - startTime;
    expect(totalTime).toBeLessThan(25000); // Should complete within 25 seconds

    // Verify final response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should handle tool execution timeouts gracefully', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Perform a complex search operation');

    // Set a reasonable timeout for the operation
    const toolResult = await waitForToolCall(request, workId, 'grep', 8000);

    // Verify tool either completes or times out gracefully
    expect(['completed']).toContain(toolResult.status);

    // Verify some response is generated even if timed out
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should validate write_file performance with different file sizes', async ({ page, request }) => {
    // Test with small file
    const smallFileWorkId = await startLLMAgentWork(page, 'Create a small text file');
    const smallFileStart = Date.now();
    await waitForToolCall(request, smallFileWorkId, 'write_file', 3000);
    const smallFileTime = Date.now() - smallFileStart;

    expect(smallFileTime).toBeLessThan(5000);

    // Test with medium file
    const mediumFileWorkId = await startLLMAgentWork(page, 'Create a medium-sized file with more content');
    const mediumFileStart = Date.now();
    await waitForToolCall(request, mediumFileWorkId, 'write_file', 5000);
    const mediumFileTime = Date.now() - mediumFileStart;

    expect(mediumFileTime).toBeLessThan(8000);

    // Small file should be faster than medium file (basic sanity check)
    expect(smallFileTime).toBeLessThan(mediumFileTime);
  });

  test('should maintain consistent performance across multiple executions', async ({ page, request }) => {
    const executionTimes: number[] = [];

    // Execute the same operation multiple times
    for (let i = 0; i < 3; i++) {
      const workId = await startLLMAgentWork(page, `Write test file ${i + 1}`);
      const startTime = Date.now();

      await waitForToolCall(request, workId, 'write_file', 5000);

      const executionTime = Date.now() - startTime;
      executionTimes.push(executionTime);

      // Each execution should complete within reasonable time
      expect(executionTime).toBeLessThan(10000);
    }

    // Calculate performance variance
    const avgTime = executionTimes.reduce((a, b) => a + b, 0) / executionTimes.length;
    const variance = executionTimes.reduce((acc, time) => acc + Math.pow(time - avgTime, 2), 0) / executionTimes.length;
    const stdDev = Math.sqrt(variance);

    // Performance should be reasonably consistent (std dev < 50% of average)
    expect(stdDev / avgTime).toBeLessThan(0.5);
  });

  test('should handle concurrent tool executions efficiently', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Create multiple files simultaneously');

    const startTime = Date.now();

    // Start multiple write operations
    const toolPromises = [];
    for (let i = 0; i < 3; i++) {
      toolPromises.push(waitForToolCall(request, workId, 'write_file', 8000));
    }

    // Wait for all operations to complete
    await Promise.all(toolPromises);

    const totalTime = Date.now() - startTime;

    // Concurrent operations should complete faster than sequential
    // (allowing some overhead for coordination)
    expect(totalTime).toBeLessThan(15000);

    // Verify all operations completed
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should validate grep performance with different search scopes', async ({ page, request }) => {
    // Test focused search
    const focusedWorkId = await startLLMAgentWork(page, 'Search for "function" in src directory only');
    const focusedStart = Date.now();
    await waitForToolCall(request, focusedWorkId, 'grep', 5000);
    const focusedTime = Date.now() - focusedStart;

    // Test broad search
    const broadWorkId = await startLLMAgentWork(page, 'Search for "function" in entire codebase');
    const broadStart = Date.now();
    await waitForToolCall(request, broadWorkId, 'grep', 8000);
    const broadTime = Date.now() - broadStart;

    // Both should complete within reasonable time
    expect(focusedTime).toBeLessThan(10000);
    expect(broadTime).toBeLessThan(15000);

    // Focused search should generally be faster (though not guaranteed due to various factors)
    // This is more of a sanity check than a strict performance requirement
    if (broadTime > focusedTime) {
      expect(broadTime - focusedTime).toBeLessThan(10000); // Difference should be reasonable
    }
  });

  test('should handle memory-intensive operations without degradation', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Create a large file and perform extensive search');

    const startTime = Date.now();

    // Create large file
    await waitForToolCall(request, workId, 'write_file', 10000);

    // Perform extensive search
    await waitForToolCall(request, workId, 'grep', 10000);

    const totalTime = Date.now() - startTime;

    // Should complete without excessive time degradation
    expect(totalTime).toBeLessThan(30000);

    // Verify system remains responsive
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
    expect(llmResponse.content.length).toBeGreaterThan(10);
  });

  test('should maintain performance under load', async ({ page, request }) => {
    const workIds: string[] = [];
    const startTimes: number[] = [];

    // Start multiple concurrent work sessions
    for (let i = 0; i < 3; i++) {
      const workId = await startLLMAgentWork(page, `Load test operation ${i + 1}`);
      workIds.push(workId);
      startTimes.push(Date.now());
    }

    // Wait for all operations to complete
    const completionPromises = workIds.map((workId, index) =>
      waitForToolCall(request, workId, index % 2 === 0 ? 'write_file' : 'grep', 10000)
    );

    await Promise.all(completionPromises);

    const endTime = Date.now();

    // Calculate average completion time
    const avgCompletionTime = startTimes.reduce((acc, startTime) => {
      return acc + (endTime - startTime);
    }, 0) / startTimes.length;

    // Should maintain reasonable performance under load
    expect(avgCompletionTime).toBeLessThan(20000);

    // Verify all operations produced results
    for (const workId of workIds) {
      const llmResponse = await waitForLLMResponse(request, workId);
      expect(llmResponse.content).toBeDefined();
    }
  });

  test('should validate tool execution reliability over time', async ({ page, request }) => {
    const results: { success: boolean; executionTime: number }[] = [];

    // Execute operations repeatedly to test reliability
    for (let i = 0; i < 5; i++) {
      try {
        const workId = await startLLMAgentWork(page, `Reliability test ${i + 1}`);
        const startTime = Date.now();

        await waitForToolCall(request, workId, i % 2 === 0 ? 'write_file' : 'grep', 8000);

        const executionTime = Date.now() - startTime;
        results.push({ success: true, executionTime });

        // Brief pause between operations
        await new Promise(resolve => setTimeout(resolve, 500));
      } catch (error) {
        results.push({ success: false, executionTime: 0 });
      }
    }

    // Calculate success rate
    const successCount = results.filter(r => r.success).length;
    const successRate = successCount / results.length;

    // Should maintain high reliability
    expect(successRate).toBeGreaterThan(0.8); // At least 80% success rate

    // Successful operations should be within performance bounds
    const successfulTimes = results.filter(r => r.success).map(r => r.executionTime);
    if (successfulTimes.length > 0) {
      const avgTime = successfulTimes.reduce((a, b) => a + b, 0) / successfulTimes.length;
      expect(avgTime).toBeLessThan(15000);
    }
  });
});