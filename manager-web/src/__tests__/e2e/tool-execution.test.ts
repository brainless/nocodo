import { expect, test } from './setup';
import {
  extractToolCallsFromWork,
  startLLMAgentWork,
  waitForLLMResponse,
  waitForToolCall
} from './test-utils';

test.describe('Tool Execution API Tests', () => {
  test('should execute list_files tool and return results via API', async ({ page, request }) => {
    // Start LLM agent work
    const workId = await startLLMAgentWork(page, 'List all files in the root directory');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'list_files');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('list_files');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message).toBeDefined();

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse).toBeDefined();
    expect(llmResponse.content).toBeDefined();
    expect(llmResponse.content.length).toBeGreaterThan(0);
  });

  test('should execute read_file tool and return content via API', async ({ page, request }) => {
    // Start LLM agent work
    const workId = await startLLMAgentWork(page, 'Read the contents of README.md');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'read_file');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('read_file');
    expect(toolCallResult.status).toBe('completed');

    // Verify LLM follow-up response contains file content
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toContain('README');
    expect(llmResponse.content.length).toBeGreaterThan(10);
  });

  test('should handle multiple sequential tool calls via API', async ({ page, request }) => {
    const prompt = 'List files in current directory, then read the README.md file';

    // Start LLM agent work
    const workId = await startLLMAgentWork(page, prompt);

    // Wait for first tool call (list_files)
    const firstToolCall = await waitForToolCall(request, workId, 'list_files');
    expect(firstToolCall.status).toBe('completed');

    // Wait for second tool call (read_file)
    const secondToolCall = await waitForToolCall(request, workId, 'read_file');
    expect(secondToolCall.status).toBe('completed');

    // Verify both tool calls are recorded
    const allToolCalls = await extractToolCallsFromWork(request, workId);
    expect(allToolCalls.length).toBeGreaterThanOrEqual(2);

    // Verify final LLM response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toContain('README');
  });

  test('should validate tool execution timing and performance', async ({ page, request }) => {
    const startTime = Date.now();

    // Start LLM agent work
    const workId = await startLLMAgentWork(page, 'List all files quickly');

    // Wait for tool execution
    const toolCallResult = await waitForToolCall(request, workId, 'list_files', 5000);
    const executionTime = Date.now() - startTime;

    // Verify execution completed within reasonable time
    expect(executionTime).toBeLessThan(10000); // Should complete within 10 seconds
    expect(executionTime).toBeGreaterThan(1000); // Should take at least 1 second (realistic)

    // Verify tool result structure
    expect(toolCallResult).toHaveProperty('timestamp');
    expect(new Date(toolCallResult.timestamp)).toBeInstanceOf(Date);
  });

  test('should handle tool execution with complex parameters', async ({ page, request }) => {
    // Start work with specific path parameter
    const workId = await startLLMAgentWork(page, 'List files in the src directory');

    // Wait for tool execution
    const toolCallResult = await waitForToolCall(request, workId, 'list_files');

    // Verify tool was called with correct context
    expect(toolCallResult.message.content).toContain('src');

    // Verify response includes directory-specific results
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should maintain tool execution history across session', async ({ page, request }) => {
    // First tool execution
    const workId1 = await startLLMAgentWork(page, 'List files');
    await waitForToolCall(request, workId1, 'list_files');

    // Navigate back and start another work session
    await page.goto('/');
    const workId2 = await startLLMAgentWork(page, 'Read README again');
    await waitForToolCall(request, workId2, 'read_file');

    // Verify both sessions maintain their own tool history
    const toolCalls1 = await extractToolCallsFromWork(request, workId1);
    const toolCalls2 = await extractToolCallsFromWork(request, workId2);

    expect(toolCalls1.length).toBeGreaterThan(0);
    expect(toolCalls2.length).toBeGreaterThan(0);
    expect(workId1).not.toBe(workId2);
  });

  test('should validate tool response data structure', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'List all files with details');

    const toolCallResult = await waitForToolCall(request, workId, 'list_files');

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
  });
});