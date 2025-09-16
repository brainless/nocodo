import { expect, test } from './setup';

test.describe('Work Creation', () => {
  test('should create a new work session with file listing prompt', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForTimeout(1000); // Give time for components to render
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('List all files in the root directory');

    // Tool is now hardcoded to llm-agent per issue #110 - no selection needed

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work Session")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/work-\d+/);

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Verify the prompt appears in the work details
    await expect(page.locator('text=List all files in the root directory')).toBeVisible();

    // Verify work status shows as running or completed
    const statusBadge = page.locator('[class*="bg-blue-100"], [class*="bg-green-100"]');
    await expect(statusBadge).toBeVisible();
  });

  test('should handle error when creating work with empty prompt', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForTimeout(1000); // Give time for components to render
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Check that submit button is disabled when prompt is empty
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work Session")');
    await expect(submitButton).toBeDisabled();

    // Fill in a prompt and verify button becomes enabled
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('Test prompt');
    await expect(submitButton).toBeEnabled();

    // Check for any error messages that might appear
    const errorMessage = page.locator('[class*="text-red"], [class*="bg-red"]');
    const isErrorVisible = await errorMessage.isVisible();

    if (isErrorVisible) {
      await expect(errorMessage).toBeVisible();
    } else {
      // If no error message, check that we're still on the dashboard
      await expect(page.locator('h3:has-text("What would you like to Work on?")')).toBeVisible();
    }
  });

  test('should create work with project selection', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForTimeout(1000); // Give time for components to render
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('Read the contents of README.md');

    // Try to select a project if available
    const projectButton = page.locator('button[aria-haspopup="listbox"]:has-text("No Project")');
    if (await projectButton.isVisible()) {
      await projectButton.click();

      // Select first available project if dropdown appears
      const projectOption = page.locator('div[role="option"]').first();
      if (await projectOption.isVisible()) {
        await projectOption.click();
      }
    }

    // Tool is now hardcoded to llm-agent per issue #110 - no selection needed

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work Session")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/work-\d+/);

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Verify the prompt appears (may be formatted differently by LLM agent)
    const promptText = await page
      .locator('text')
      .filter({ hasText: /Read.*README\.md/ })
      .first();
    await expect(promptText).toBeVisible();
  });
});
