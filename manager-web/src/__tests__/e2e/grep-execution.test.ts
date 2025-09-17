import { expect, test } from './setup';
import {
  extractToolCallsFromWork,
  startLLMAgentWork,
  waitForLLMResponse,
  waitForToolCall,
} from './test-utils';

test.describe('Grep Tool Execution API Tests', () => {
  test('should execute grep search via API', async ({ page, request }) => {
    // Start LLM agent work
    const workId = await startLLMAgentWork(page, 'Search for "function" in all files');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('grep');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message).toBeDefined();

    // Verify tool result structure
    expect(toolCallResult.message.content).toContain('matches');
    expect(toolCallResult.message.content).toContain('files_searched');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse).toBeDefined();
    expect(llmResponse.content).toBeDefined();
    expect(llmResponse.content.length).toBeGreaterThan(0);
  });

  test('should handle regex patterns in grep tool', async ({ page, request }) => {
    // Start work with regex pattern
    const workId = await startLLMAgentWork(page, 'Search for pattern "fn\\s+\\w+" in Rust files');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('grep');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message.content).toContain('pattern');
    expect(toolCallResult.message.content).toMatch(/(line_number|match_start|fn\s+\w+)/);

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toContain('fn');
  });

  test('should search specific directories via grep tool', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Search for "struct" only in the src directory');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('grep');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message.content).toContain('src');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should respect max_results parameter in grep tool', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Search for "use" with maximum 5 results');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('grep');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message.content).toContain('total_matches');
    expect(toolCallResult.message.content).toContain('truncated');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should handle case-sensitive grep searches', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Search for "Function" with case sensitivity');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('grep');
    expect(toolCallResult.status).toBe('completed');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should handle grep searches with include patterns', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Search for "test" only in TypeScript files');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('grep');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message.content).toContain('.ts');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should handle grep searches with exclude patterns', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Search for "log" but exclude node_modules');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('grep');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message.content).not.toContain('node_modules');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should validate grep tool execution timing and performance', async ({ page, request }) => {
    const startTime = Date.now();

    // Start LLM agent work
    const workId = await startLLMAgentWork(page, 'Search for "const" in source files');

    // Wait for tool execution
    const toolCallResult = await waitForToolCall(request, workId, 'grep', 5000);
    const executionTime = Date.now() - startTime;

    // Verify execution completed within reasonable time
    expect(executionTime).toBeLessThan(15000); // Should complete within 15 seconds
    expect(executionTime).toBeGreaterThan(1000); // Should take at least 1 second (realistic)

    // Verify tool result structure
    expect(toolCallResult).toHaveProperty('timestamp');
    expect(new Date(toolCallResult.timestamp)).toBeInstanceOf(Date);
  });

  test('should handle grep searches in nested directories', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Search for "export" in all subdirectories');

    // Wait for tool execution
    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool was called with recursive search
    expect(toolCallResult.message.content).toContain('files_searched');

    // Verify response includes nested directory results
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should maintain grep tool execution history across sessions', async ({ page, request }) => {
    // First grep search
    const workId1 = await startLLMAgentWork(page, 'Search for "import" statements');
    await waitForToolCall(request, workId1, 'grep');

    // Navigate back and start another work session
    await page.goto('/');
    const workId2 = await startLLMAgentWork(page, 'Search for "class" definitions');
    await waitForToolCall(request, workId2, 'grep');

    // Verify both sessions maintain their own tool history
    const toolCalls1 = await extractToolCallsFromWork(request, workId1);
    const toolCalls2 = await extractToolCallsFromWork(request, workId2);

    expect(toolCalls1.length).toBeGreaterThan(0);
    expect(toolCalls2.length).toBeGreaterThan(0);
    expect(workId1).not.toBe(workId2);
  });

  test('should validate grep tool response data structure', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Search for "async" in the codebase');

    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool result has expected structure
    expect(toolCallResult).toHaveProperty('workId');
    expect(toolCallResult).toHaveProperty('toolType');
    expect(toolCallResult).toHaveProperty('status');
    expect(toolCallResult).toHaveProperty('message');
    expect(toolCallResult).toHaveProperty('timestamp');

    // Verify message structure
    expect(toolCallResult.message).toHaveProperty('id');
    expect(toolCallResult.message).toHaveProperty('content');
    expect(toolCallResult.message).toHaveProperty('created_at');

    // Verify grep specific fields in content
    expect(toolCallResult.message.content).toMatch(
      /(matches|files_searched|total_matches|pattern)/
    );
  });

  test('should handle grep searches with no results', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Search for "nonexistent_pattern_xyz123"');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('grep');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message.content).toContain('0');
    expect(toolCallResult.message.content).toContain('matches');

    // Verify LLM follow-up response handles no results gracefully
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should handle grep searches with special characters', async ({ page, request }) => {
    const workId = await startLLMAgentWork(
      page,
      'Search for patterns with special characters like "->" or "::"'
    );

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'grep');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('grep');
    expect(toolCallResult.status).toBe('completed');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });
});
