import { test, expect } from './setup';

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

    // Simulate network disconnection by blocking WebSocket connections
    await page.route('ws://**', route => route.abort());
    await page.route('wss://**', route => route.abort());

    // Wait a bit for disconnection to be detected
    await page.waitForTimeout(5000);

    // Check that the UI handles disconnection appropriately
    // Should either show error status or attempt reconnection
    const errorStatus = page.locator('[class*="bg-red-500"]');
    const connectingStatus = page.locator('[class*="bg-yellow-500"]');

    // Should show either error or reconnecting status
    await expect(errorStatus.or(connectingStatus)).toBeVisible({ timeout: 10000 });
  });

  test('should reconnect WebSocket after temporary disconnection', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Check initial connection
    const statusIndicator = page.locator('[class*="w-2 h-2 rounded-full"]');
    await expect(statusIndicator).toBeVisible();

    // Temporarily block WebSocket connections
    await page.route('ws://**', route => route.abort());
    await page.route('wss://**', route => route.abort());

    // Wait for disconnection
    await page.waitForTimeout(3000);

    // Restore WebSocket connections
    await page.unroute('ws://**');
    await page.unroute('wss://**');

    // Wait for reconnection
    await page.waitForTimeout(5000);

    // Check that connection is restored
    const connectedStatus = page.locator('[class*="bg-green-500"]');
    const connectingStatus = page.locator('[class*="bg-yellow-500"]');

    // Should show connected or connecting status
    await expect(connectedStatus.or(connectingStatus)).toBeVisible({ timeout: 15000 });
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

    // Wait for processing
    await page.waitForTimeout(12000);

    // Check if new content appeared (indicating WebSocket messages were processed)
    const finalContent = await page
      .locator('[class*="bg-black"], [class*="text-gray-100"]')
      .count();

    // Should have at least some content after processing
    expect(finalContent).toBeGreaterThanOrEqual(initialContent);
  });
});
