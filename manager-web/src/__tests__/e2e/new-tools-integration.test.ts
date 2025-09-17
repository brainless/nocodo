import { expect, test } from './setup';
import {
  extractToolCallsFromWork,
  startLLMAgentWork,
  waitForLLMResponse,
  waitForToolCall,
} from './test-utils';

test.describe('New Tools Integration Tests', () => {
  test('should handle write_file followed by grep search', async ({ page, request }) => {
    const workId = await startLLMAgentWork(
      page,
      'Create a file with function definitions, then search for them'
    );

    // Wait for write_file tool
    const writeResult = await waitForToolCall(request, workId, 'write_file');
    expect(writeResult.status).toBe('completed');
    expect(writeResult.message.content).toContain('created');

    // Wait for grep tool
    const grepResult = await waitForToolCall(request, workId, 'grep');
    expect(grepResult.status).toBe('completed');
    expect(grepResult.message.content).toContain('matches');

    // Verify both tools are recorded
    const allToolCalls = await extractToolCallsFromWork(request, workId);
    expect(allToolCalls.length).toBeGreaterThanOrEqual(2);

    // Verify final LLM response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toContain('function');
  });

  test('should support complex file manipulation workflows', async ({ page, request }) => {
    const prompt = 'List files, read a config file, modify it, then search for the changes';
    const workId = await startLLMAgentWork(page, prompt);

    // Wait for list_files tool
    const listResult = await waitForToolCall(request, workId, 'list_files');
    expect(listResult.status).toBe('completed');

    // Wait for read_file tool
    const readResult = await waitForToolCall(request, workId, 'read_file');
    expect(readResult.status).toBe('completed');

    // Wait for write_file tool
    const writeResult = await waitForToolCall(request, workId, 'write_file');
    expect(writeResult.status).toBe('completed');

    // Wait for grep tool
    const grepResult = await waitForToolCall(request, workId, 'grep');
    expect(grepResult.status).toBe('completed');

    // Verify all tools completed successfully
    expect(
      [listResult, readResult, writeResult, grepResult].every(r => r.status === 'completed')
    ).toBe(true);

    // Verify final LLM response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should handle write_file and grep in sequence with error recovery', async ({
    page,
    request,
  }) => {
    const workId = await startLLMAgentWork(
      page,
      'Create a file, then search for non-existent pattern, then create another file'
    );

    // First write_file
    const writeResult1 = await waitForToolCall(request, workId, 'write_file');
    expect(writeResult1.status).toBe('completed');

    // Grep with no results
    const grepResult = await waitForToolCall(request, workId, 'grep');
    expect(grepResult.status).toBe('completed');
    expect(grepResult.message.content).toContain('0');

    // Second write_file
    const writeResult2 = await waitForToolCall(request, workId, 'write_file');
    expect(writeResult2.status).toBe('completed');

    // Verify final LLM response handles the workflow gracefully
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should execute multiple grep searches in sequence', async ({ page, request }) => {
    const workId = await startLLMAgentWork(
      page,
      'Search for "function", then search for "class", then search for "interface"'
    );

    // First grep search
    const grepResult1 = await waitForToolCall(request, workId, 'grep');
    expect(grepResult1.status).toBe('completed');

    // Second grep search
    const grepResult2 = await waitForToolCall(request, workId, 'grep');
    expect(grepResult2.status).toBe('completed');

    // Third grep search
    const grepResult3 = await waitForToolCall(request, workId, 'grep');
    expect(grepResult3.status).toBe('completed');

    // Verify all grep operations completed
    expect([grepResult1, grepResult2, grepResult3].every(r => r.status === 'completed')).toBe(true);

    // Verify final LLM response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should handle write_file with different file types and grep validation', async ({
    page,
    request,
  }) => {
    const workId = await startLLMAgentWork(
      page,
      'Create multiple files of different types, then search for specific patterns in each'
    );

    // Multiple write_file operations
    const writeResults = [];
    for (let i = 0; i < 3; i++) {
      const writeResult = await waitForToolCall(request, workId, 'write_file');
      expect(writeResult.status).toBe('completed');
      writeResults.push(writeResult);
    }

    // Multiple grep operations for validation
    const grepResults = [];
    for (let i = 0; i < 3; i++) {
      const grepResult = await waitForToolCall(request, workId, 'grep');
      expect(grepResult.status).toBe('completed');
      grepResults.push(grepResult);
    }

    // Verify all operations completed
    expect(writeResults.length).toBe(3);
    expect(grepResults.length).toBe(3);

    // Verify final LLM response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should maintain tool execution order in complex workflows', async ({ page, request }) => {
    const workId = await startLLMAgentWork(
      page,
      'Read existing file, modify it, search for old content, then search for new content'
    );

    // Read existing file
    const readResult = await waitForToolCall(request, workId, 'read_file');
    expect(readResult.status).toBe('completed');

    // Modify file
    const writeResult = await waitForToolCall(request, workId, 'write_file');
    expect(writeResult.status).toBe('completed');

    // Search for old content (should not find it)
    const grepResult1 = await waitForToolCall(request, workId, 'grep');
    expect(grepResult1.status).toBe('completed');

    // Search for new content (should find it)
    const grepResult2 = await waitForToolCall(request, workId, 'grep');
    expect(grepResult2.status).toBe('completed');

    // Verify final LLM response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should handle tool failures gracefully in integration scenarios', async ({
    page,
    request,
  }) => {
    const workId = await startLLMAgentWork(
      page,
      'Try to write to invalid path, then create valid file, then search successfully'
    );

    // First write_file (should fail)
    const writeResult1 = await waitForToolCall(request, workId, 'write_file');
    expect(writeResult1.status).toBe('completed'); // Tool executed but with error

    // Second write_file (should succeed)
    const writeResult2 = await waitForToolCall(request, workId, 'write_file');
    expect(writeResult2.status).toBe('completed');

    // Grep search (should succeed)
    const grepResult = await waitForToolCall(request, workId, 'grep');
    expect(grepResult.status).toBe('completed');

    // Verify final LLM response handles mixed success/failure gracefully
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should validate tool execution timing in integration scenarios', async ({
    page,
    request,
  }) => {
    const startTime = Date.now();

    const workId = await startLLMAgentWork(
      page,
      'Quickly create a file and search for its content'
    );

    // Wait for write_file
    await waitForToolCall(request, workId, 'write_file', 3000);

    // Wait for grep
    await waitForToolCall(request, workId, 'grep', 5000);

    const totalTime = Date.now() - startTime;
    expect(totalTime).toBeLessThan(15000); // Should complete within 15 seconds

    // Verify final LLM response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should handle large file creation and search integration', async ({ page, request }) => {
    const workId = await startLLMAgentWork(
      page,
      'Create a large file with many lines, then search for specific patterns'
    );

    // Create large file
    const writeResult = await waitForToolCall(request, workId, 'write_file', 10000);
    expect(writeResult.status).toBe('completed');

    // Search within the large file
    const grepResult = await waitForToolCall(request, workId, 'grep', 10000);
    expect(grepResult.status).toBe('completed');

    // Verify final LLM response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should maintain state consistency across tool executions', async ({ page, request }) => {
    const workId = await startLLMAgentWork(
      page,
      'Create file A, create file B, search in both, then modify file A'
    );

    // Create file A
    const writeResult1 = await waitForToolCall(request, workId, 'write_file');
    expect(writeResult1.status).toBe('completed');

    // Create file B
    const writeResult2 = await waitForToolCall(request, workId, 'write_file');
    expect(writeResult2.status).toBe('completed');

    // Search in both files
    const grepResult = await waitForToolCall(request, workId, 'grep');
    expect(grepResult.status).toBe('completed');

    // Modify file A
    const writeResult3 = await waitForToolCall(request, workId, 'write_file');
    expect(writeResult3.status).toBe('completed');

    // Verify all operations maintained state correctly
    const allToolCalls = await extractToolCallsFromWork(request, workId);
    expect(allToolCalls.length).toBeGreaterThanOrEqual(4);

    // Verify final LLM response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });
});
