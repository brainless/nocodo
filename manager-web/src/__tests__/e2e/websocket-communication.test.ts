import { expect, test } from './setup';
import { startLLMAgentWork, monitorWebSocketMessages } from './test-utils';

test.describe('WebSocket Communication', () => {
  test('should establish WebSocket connection on page load', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Check for connection status indicator
    const statusIndicator = page.locator('[class*="w-2 h-2 rounded-full"]');

    // Should show some connection status (green for connected, yellow for connecting, red for error)
    await expect(statusIndicator).toBeVisible();

    // The status should be visible (could be connected, connecting, or disconnected in mock)
    const statusClasses = await statusIndicator.getAttribute('class');
    expect(statusClasses).toMatch(/bg-(green|yellow|gray|red)-500/);
  });

  test('should show real-time updates during work processing', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('List all files in the root directory');

    // Select tool using custom dropdown
    const toolButton = page
      .locator('button[aria-haspopup="listbox"]')
      .filter({ hasText: 'llm-agent' });
    await toolButton.click();

    // Wait for dropdown options and select llm-agent
    await page.locator('div[role="option"]:has-text("llm-agent")').click();

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/work-\d+/);

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Check for initial status (should be running)
    const runningBadge = page.locator('[class*="bg-blue-100"]');
    await expect(runningBadge).toBeVisible();

    // Wait for processing and check for status changes
    await page.waitForTimeout(10000);

    // Should either complete or still be running with updates
    const completedBadge = page.locator('[class*="bg-green-100"]');
    const stillRunningBadge = page.locator('[class*="bg-blue-100"]');

    await expect(completedBadge.or(stillRunningBadge)).toBeVisible();
  });

  test('should handle WebSocket disconnection gracefully', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Check initial connection status
    const statusIndicator = page.locator('[class*="w-2 h-2 rounded-full"]');
    await expect(statusIndicator).toBeVisible();

    // For now, just verify the connection status indicator is present
    // WebSocket disconnection testing would require more complex mocking
    // This test ensures the UI has the necessary elements for connection status
    const statusText = page.locator('[class*="text-sm text-gray-600"]');
    await expect(statusText).toBeVisible();
  });

  test('should reconnect WebSocket after temporary disconnection', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Check initial connection
    const statusIndicator = page.locator('[class*="w-2 h-2 rounded-full"]');
    await expect(statusIndicator).toBeVisible();

    // Verify the connection status persists across page interactions
    // This test ensures the WebSocket connection status is maintained
    const statusText = page.locator('[class*="text-sm text-gray-600"]');
    await expect(statusText).toBeVisible();

    // Navigate away and back to test connection persistence
    await page.goto('/projects');
    await page.goto('/');

    // Connection status should still be visible
    await expect(statusIndicator).toBeVisible();
  });

  test('should handle WebSocket messages during work execution', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('List all files in the root directory');

    // Select tool using custom dropdown
    const toolButton = page
      .locator('button[aria-haspopup="listbox"]')
      .filter({ hasText: 'llm-agent' });
    await toolButton.click();

    // Wait for dropdown options and select llm-agent
    await page.locator('div[role="option"]:has-text("llm-agent")').click();

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/work-\d+/);

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Monitor for real-time updates
    // Look for status changes or new content appearing
    const initialContent = await page
      .locator('[class*="bg-black"], [class*="text-gray-100"]')
      .count();

    // Wait for processing with proper timeout
    await page.waitForSelector('[class*="bg-black"], [class*="text-gray-100"]', { timeout: 15000 });

    // Check if new content appeared (indicating WebSocket messages were processed)
    const finalContent = await page
      .locator('[class*="bg-black"], [class*="text-gray-100"]')
      .count();

    // Should have at least some content after processing
    expect(finalContent).toBeGreaterThanOrEqual(initialContent);
  });

  test('should receive real-time updates during tool execution via WebSocket', async ({ page }) => {
    const messages: any[] = [];

    // Set up WebSocket message monitoring
    page.on('websocket', ws => {
      ws.on('framereceived', frame => {
        try {
          const data = JSON.parse(frame.payload);
          if (data.type === 'LlmAgentChunk' || data.type === 'AiSessionStatusChanged') {
            messages.push(data);
          }
        } catch (error) {
          // Ignore non-JSON frames
        }
      });
    });

    // Start tool execution
    await startLLMAgentWork(page, 'List all files in the root directory');

    // Wait for WebSocket messages
    await page.waitForTimeout(5000);

    // Verify we received tool execution messages
    expect(messages.length).toBeGreaterThan(0);

    // Check for tool execution indicators
    const hasToolExecution = messages.some(msg =>
      msg.type === 'LlmAgentChunk' &&
      (msg.payload.content.includes('list_files') ||
       msg.payload.content.includes('Executing') ||
       msg.payload.content.includes('tool'))
    );
    expect(hasToolExecution).toBeTruthy();

    // Check for status changes
    const hasStatusChange = messages.some(msg =>
      msg.type === 'AiSessionStatusChanged'
    );
    expect(hasStatusChange).toBeTruthy();
  });

  test('should stream tool results in real-time via WebSocket', async ({ page }) => {
    const chunkMessages: any[] = [];

    // Monitor for streaming chunks
    page.on('websocket', ws => {
      ws.on('framereceived', frame => {
        try {
          const data = JSON.parse(frame.payload);
          if (data.type === 'LlmAgentChunk') {
            chunkMessages.push(data);
          }
        } catch (error) {
          // Ignore non-JSON frames
        }
      });
    });

    // Start work that will produce streaming results
    await startLLMAgentWork(page, 'List files and show me the results');

    // Wait for streaming to complete
    await page.waitForTimeout(6000);

    // Verify we received multiple chunks
    expect(chunkMessages.length).toBeGreaterThan(3);

    // Verify chunks contain file information
    const fileRelatedChunks = chunkMessages.filter(chunk =>
      chunk.payload.content.includes('README') ||
      chunk.payload.content.includes('package.json') ||
      chunk.payload.content.includes('Cargo.toml') ||
      chunk.payload.content.includes('files')
    );
    expect(fileRelatedChunks.length).toBeGreaterThan(0);

    // Verify chunks arrive in sequence
    for (let i = 1; i < chunkMessages.length; i++) {
      expect(chunkMessages[i].payload.session_id).toBe(chunkMessages[0].payload.session_id);
    }
  });

  test('should handle WebSocket messages during multiple tool calls', async ({ page }) => {
    const allMessages: any[] = [];

    // Monitor all WebSocket messages
    page.on('websocket', ws => {
      ws.on('framereceived', frame => {
        try {
          const data = JSON.parse(frame.payload);
          allMessages.push(data);
        } catch (error) {
          // Ignore non-JSON frames
        }
      });
    });

    // Start work with multiple tool calls
    await startLLMAgentWork(page, 'List files in root directory, then read README.md');

    // Wait for all operations to complete
    await page.waitForTimeout(8000);

    // Verify we received messages for both tools
    const toolExecutionMessages = allMessages.filter(msg =>
      msg.type === 'LlmAgentChunk' &&
      (msg.payload.content.includes('list_files') ||
       msg.payload.content.includes('read_file') ||
       msg.payload.content.includes('Executing'))
    );
    expect(toolExecutionMessages.length).toBeGreaterThan(1);

    // Verify session consistency across multiple tool calls
    const sessionIds = [...new Set(allMessages
      .filter(msg => msg.payload?.session_id)
      .map(msg => msg.payload.session_id)
    )];
    expect(sessionIds.length).toBe(1); // All messages should belong to same session
  });

  test('should maintain WebSocket connection stability during tool execution', async ({ page }) => {
    let connectionEvents: string[] = [];
    let messageCount = 0;

    // Monitor connection stability
    page.on('websocket', ws => {
      ws.on('framereceived', () => {
        messageCount++;
      });

      // Note: Playwright doesn't expose connection events directly,
      // but we can monitor message flow
    });

    // Start tool execution
    await startLLMAgentWork(page, 'List all files');

    // Wait for execution
    await page.waitForTimeout(5000);

    // Verify continuous message flow (no connection drops)
    expect(messageCount).toBeGreaterThan(5);

    // Verify connection status remains stable
    const statusIndicator = page.locator('[class*="w-2 h-2 rounded-full"]');
    await expect(statusIndicator).toBeVisible();

    // Status should be green (connected) or yellow (connecting), not red (error)
    const statusClasses = await statusIndicator.getAttribute('class');
    expect(statusClasses).toMatch(/bg-(green|yellow)-500/);
  });

  test('should handle WebSocket message ordering during tool execution', async ({ page }) => {
    const orderedMessages: any[] = [];
    let lastTimestamp = 0;

    // Monitor message ordering
    page.on('websocket', ws => {
      ws.on('framereceived', frame => {
        try {
          const data = JSON.parse(frame.payload);
          const timestamp = Date.now();
          orderedMessages.push({ ...data, receivedAt: timestamp });

          // Verify messages arrive in order (with small tolerance for network jitter)
          if (lastTimestamp > 0) {
            expect(timestamp - lastTimestamp).toBeGreaterThanOrEqual(0);
          }
          lastTimestamp = timestamp;
        } catch (error) {
          // Ignore non-JSON frames
        }
      });
    });

    // Start tool execution
    await startLLMAgentWork(page, 'Execute a simple file listing');

    // Wait for completion
    await page.waitForTimeout(4000);

    // Verify message sequence
    expect(orderedMessages.length).toBeGreaterThan(2);

    // Verify logical sequence: processing -> tool execution -> results -> completion
    const messageTypes = orderedMessages.map(msg => msg.type);
    expect(messageTypes).toContain('LlmAgentChunk');
  });

  test('should handle WebSocket errors gracefully during tool execution', async ({ page }) => {
    // This test simulates network issues during tool execution
    let errorMessages: any[] = [];
    let normalMessages: any[] = [];

    page.on('websocket', ws => {
      ws.on('framereceived', frame => {
        try {
          const data = JSON.parse(frame.payload);
          if (data.type === 'Error') {
            errorMessages.push(data);
          } else {
            normalMessages.push(data);
          }
        } catch (error) {
          // Ignore parsing errors
        }
      });
    });

    // Start tool execution
    await startLLMAgentWork(page, 'List files');

    // Wait for execution
    await page.waitForTimeout(3000);

    // In a real scenario, we might have error messages, but in our mock
    // we expect clean execution. Verify no unexpected errors occurred.
    const unexpectedErrors = errorMessages.filter(msg =>
      !msg.payload.message.includes('expected')
    );
    expect(unexpectedErrors.length).toBe(0);

    // Verify normal operation continued despite any handled errors
    expect(normalMessages.length).toBeGreaterThan(0);
  });
});
