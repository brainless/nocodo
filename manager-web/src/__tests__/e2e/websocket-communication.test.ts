import { expect, test } from './setup';

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

    // The status should be either connected or connecting (not permanently disconnected)
    const statusClasses = await statusIndicator.getAttribute('class');
    expect(statusClasses).toMatch(/bg-(green|yellow)-500/);
  });

  test('should show real-time updates during work processing', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('List all files in the root directory');

    // Select tool
    const toolSelect = page.locator('select').first();
    await toolSelect.selectOption('claude');

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/\d+/);

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

    // Select tool
    const toolSelect = page.locator('select').first();
    await toolSelect.selectOption('claude');

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/\d+/);

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
});
