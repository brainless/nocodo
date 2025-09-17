import { expect, test } from './setup';
import {
  extractToolCallsFromWork,
  startLLMAgentWork,
  waitForLLMResponse,
  waitForToolCall,
} from './test-utils';

test.describe('Write File Tool Execution API Tests', () => {
  test('should create new file via write_file tool API', async ({ page, request }) => {
    // Start LLM agent work
    const workId = await startLLMAgentWork(page, 'Create a new file called test.txt with content "Hello World"');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'write_file');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('write_file');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message).toBeDefined();

    // Verify tool result structure
    expect(toolCallResult.message.content).toContain('test.txt');
    expect(toolCallResult.message.content).toContain('created');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse).toBeDefined();
    expect(llmResponse.content).toBeDefined();
    expect(llmResponse.content.length).toBeGreaterThan(0);
  });

  test('should overwrite existing file via write_file tool API', async ({ page, request }) => {
    // First create a file
    const createWorkId = await startLLMAgentWork(page, 'Create a file called overwrite-test.txt with content "Original content"');
    await waitForToolCall(request, createWorkId, 'write_file');

    // Now overwrite it
    const workId = await startLLMAgentWork(page, 'Overwrite overwrite-test.txt with new content "Updated content"');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'write_file');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('write_file');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message.content).toContain('overwrite-test.txt');
    expect(toolCallResult.message.content).toContain('bytes_written');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toContain('Updated content');
  });

  test('should perform search and replace via write_file tool API', async ({ page, request }) => {
    // First create a file with specific content
    const createWorkId = await startLLMAgentWork(page, 'Create config.toml with content "[database]\nurl = \"old_url\"\nport = 5432"');
    await waitForToolCall(request, createWorkId, 'write_file');

    // Now perform search and replace
    const workId = await startLLMAgentWork(page, 'Replace "old_url" with "new_url" in config.toml');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'write_file');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('write_file');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message.content).toContain('config.toml');
    expect(toolCallResult.message.content).toContain('replaced');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toContain('new_url');
  });

  test('should handle write_file errors gracefully', async ({ page, request }) => {
    // Try to write to a protected system path
    const workId = await startLLMAgentWork(page, 'Write content to /protected/system/file.txt');

    // Wait for tool execution to complete (should fail)
    const toolCallResult = await waitForToolCall(request, workId, 'write_file');

    // Verify tool call was recorded but failed
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('write_file');
    expect(toolCallResult.status).toBe('completed'); // Tool executed but with error
    expect(toolCallResult.message.content).toContain('permission denied');

    // Verify error handling in LLM response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should append content to existing file via write_file tool', async ({ page, request }) => {
    // First create a file
    const createWorkId = await startLLMAgentWork(page, 'Create append-test.txt with content "Line 1\n"');
    await waitForToolCall(request, createWorkId, 'write_file');

    // Now append to it
    const workId = await startLLMAgentWork(page, 'Append "Line 2\nLine 3" to append-test.txt');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'write_file');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('write_file');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message.content).toContain('append-test.txt');
    expect(toolCallResult.message.content).toContain('bytes_written');

    // Verify LLM follow-up response contains both original and appended content
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toContain('Line 1');
    expect(llmResponse.content).toContain('Line 2');
    expect(llmResponse.content).toContain('Line 3');
  });

  test('should create directories when requested via write_file tool', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Create a file at nested/deep/path/test.txt with content "Nested file"');

    // Wait for tool execution to complete
    const toolCallResult = await waitForToolCall(request, workId, 'write_file');

    // Verify tool call was recorded
    expect(toolCallResult).toBeDefined();
    expect(toolCallResult.toolType).toBe('write_file');
    expect(toolCallResult.status).toBe('completed');
    expect(toolCallResult.message.content).toContain('nested/deep/path/test.txt');
    expect(toolCallResult.message.content).toContain('created');

    // Verify LLM follow-up response
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toContain('Nested file');
  });

  test('should validate write_file tool execution timing and performance', async ({ page, request }) => {
    const startTime = Date.now();

    // Start LLM agent work
    const workId = await startLLMAgentWork(page, 'Write a small file with content "Performance test"');

    // Wait for tool execution
    const toolCallResult = await waitForToolCall(request, workId, 'write_file', 5000);
    const executionTime = Date.now() - startTime;

    // Verify execution completed within reasonable time
    expect(executionTime).toBeLessThan(10000); // Should complete within 10 seconds
    expect(executionTime).toBeGreaterThan(1000); // Should take at least 1 second (realistic)

    // Verify tool result structure
    expect(toolCallResult).toHaveProperty('timestamp');
    expect(new Date(toolCallResult.timestamp)).toBeInstanceOf(Date);
  });

  test('should handle write_file with complex file paths', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Create a file at src/components/Button.tsx with React component code');

    // Wait for tool execution
    const toolCallResult = await waitForToolCall(request, workId, 'write_file');

    // Verify tool was called with correct context
    expect(toolCallResult.message.content).toContain('src/components/Button.tsx');

    // Verify response includes file creation results
    const llmResponse = await waitForLLMResponse(request, workId);
    expect(llmResponse.content).toBeDefined();
  });

  test('should maintain write_file tool execution history across sessions', async ({ page, request }) => {
    // First file write
    const workId1 = await startLLMAgentWork(page, 'Create file1.txt with content "First file"');
    await waitForToolCall(request, workId1, 'write_file');

    // Navigate back and start another work session
    await page.goto('/');
    const workId2 = await startLLMAgentWork(page, 'Create file2.txt with content "Second file"');
    await waitForToolCall(request, workId2, 'write_file');

    // Verify both sessions maintain their own tool history
    const toolCalls1 = await extractToolCallsFromWork(request, workId1);
    const toolCalls2 = await extractToolCallsFromWork(request, workId2);

    expect(toolCalls1.length).toBeGreaterThan(0);
    expect(toolCalls2.length).toBeGreaterThan(0);
    expect(workId1).not.toBe(workId2);
  });

  test('should validate write_file tool response data structure', async ({ page, request }) => {
    const workId = await startLLMAgentWork(page, 'Create validation-test.txt with some content');

    const toolCallResult = await waitForToolCall(request, workId, 'write_file');

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

    // Verify write_file specific fields in content
    expect(toolCallResult.message.content).toContain('validation-test.txt');
    expect(toolCallResult.message.content).toMatch(/(created|bytes_written|modified)/);
  });
});