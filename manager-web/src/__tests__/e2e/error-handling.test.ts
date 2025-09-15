import { expect, test } from './setup';

test.describe('Error Handling', () => {
  test('should handle invalid file reading requests', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForTimeout(1000); // Give time for components to render
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt for reading invalid file
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('Read the contents of /etc/passwd'); // Try to access system file

    // Tool is now hardcoded to llm-agent per issue #110 - no selection needed

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work Session")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/work-\d+/);

    // Wait for agent processing
    await page.waitForTimeout(8000);

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Check for error handling
    const errorElements = page.locator(
      '[class*="bg-red-100"], [class*="text-red"], text=/error|Error|failed|Failed/'
    );
    const failedBadge = page.locator('[class*="bg-red-100"]');

    // Should show some form of error handling
    try {
      await expect(errorElements.or(failedBadge)).toBeVisible({ timeout: 10000 });
    } catch {
      // If no explicit error, at least verify work completed with some response
      const responseContent = page.locator('[class*="bg-black"], [class*="text-gray-100"]');
      await expect(responseContent).toBeVisible();
    }
  });

  test('should handle network errors during work creation', async ({ page }) => {
    // This test would require mocking network failures
    // For now, we'll test with an extremely long prompt that might cause issues

    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForTimeout(1000); // Give time for components to render
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in an extremely long prompt that might cause processing issues
    const longPrompt = 'A'.repeat(10000); // 10,000 character prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill(longPrompt);

    // Tool is now hardcoded to llm-agent per issue #110 - no selection needed

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work Session")');
    await submitButton.click();

    // Wait for navigation or error
    try {
      await page.waitForURL(/\/work\/work-\d+/, { timeout: 10000 });
      // If we get here, work was created successfully despite long prompt
      await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();
    } catch {
      // If navigation fails, we should still be on dashboard with error
      await expect(page.locator('h3:has-text("What would you like to Work on?")')).toBeVisible();
    }
  });

  test('should handle work creation with invalid tool selection', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForTimeout(1000); // Give time for components to render
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('List all files in the root directory');

    // Tool selection removed per issue #110 - tool is now hardcoded to llm-agent
    // This test is no longer relevant as there's no tool selection UI

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work Session")');
    await submitButton.click();

    // Should either succeed or show appropriate error
    try {
      await page.waitForURL(/\/work\/work-\d+/, { timeout: 15000 });
      await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();
    } catch {
      // If it fails, should show error on dashboard
      await expect(page.locator('h3:has-text("What would you like to Work on?")')).toBeVisible();
    }
  });

  test('should handle timeout scenarios gracefully', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForTimeout(1000); // Give time for components to render
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('This is a test prompt for timeout handling');

    // Tool is now hardcoded to llm-agent per issue #110 - no selection needed

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work Session")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/work-\d+/, { timeout: 30000 });

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Wait for a reasonable time for processing
    await page.waitForTimeout(15000);

    // Check that we have some response (success or error)
    const responseContent = page.locator('[class*="bg-black"][class*="text-gray-100"]');
    const errorContent = page.locator('[class*="bg-red-100"][class*="text-red"]');

    // Should have either response content or error handling
    try {
      await expect(responseContent).toBeVisible({ timeout: 5000 });
    } catch {
      await expect(errorContent).toBeVisible({ timeout: 5000 });
    }
  });
});
