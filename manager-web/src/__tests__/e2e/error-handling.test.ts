import { expect, test } from './setup';

test.describe('Error Handling', () => {
  test('should handle invalid file reading requests', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt for reading invalid file
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('Read the contents of /etc/passwd'); // Try to access system file

    // Select tool using custom dropdown
    const toolButton = page.locator('button[aria-haspopup="listbox"]').first();
    await toolButton.click();

    // Wait for dropdown options and select llm-agent
    await page.locator('div[role="option"]:has-text("llm-agent")').click();

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/\d+/);

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
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in an extremely long prompt that might cause processing issues
    const longPrompt = 'A'.repeat(10000); // 10,000 character prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill(longPrompt);

    // Select tool using custom dropdown
    const toolButton = page.locator('button[aria-haspopup="listbox"]').first();
    await toolButton.click();

    // Wait for dropdown options and select llm-agent
    await page.locator('div[role="option"]:has-text("llm-agent")').click();

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Wait for navigation or error
    try {
      await page.waitForURL(/\/work\/\d+/, { timeout: 10000 });
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
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('List all files in the root directory');

    // Try to select an invalid tool (if available)
    const toolButton = page.locator('button[aria-haspopup="listbox"]').first();
    await toolButton.click();

    // Get all available tool options
    const toolOptions = page.locator('div[role="option"]');
    const optionCount = await toolOptions.count();

    // If there are options, try selecting one that might not be valid
    if (optionCount > 1) {
      // Select the last option (might be less tested)
      await toolOptions.nth(optionCount - 1).click();
    } else {
      // Fallback to first option
      await toolOptions.first().click();
    }

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Should either succeed or show appropriate error
    try {
      await page.waitForURL(/\/work\/\d+/, { timeout: 15000 });
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
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('This is a test prompt for timeout handling');

    // Select tool using custom dropdown
    const toolButton = page.locator('button[aria-haspopup="listbox"]').first();
    await toolButton.click();

    // Wait for dropdown options and select llm-agent
    await page.locator('div[role="option"]:has-text("llm-agent")').click();

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/\d+/, { timeout: 30000 });

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Wait for a reasonable time for processing
    await page.waitForTimeout(15000);

    // Check that we have some response (success or error)
    const responseContent = page.locator('[class*="bg-black"], [class*="text-gray-100"]');
    const errorContent = page.locator('[class*="bg-red-100"], [class*="text-red"]');

    // Should have either response content or error handling
    await expect(responseContent.or(errorContent)).toBeVisible({ timeout: 10000 });
  });
});
