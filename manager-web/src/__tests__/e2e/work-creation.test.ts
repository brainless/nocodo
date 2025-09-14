import { expect, test } from './setup';

test.describe('Work Creation', () => {
  test('should create a new work session with file listing prompt', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('List all files in the root directory');

    // Select a tool (default should be llm-agent)
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
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Try to submit without filling the prompt
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Should show validation error or prevent submission
    // The form should either show an error or the submit button should be disabled
    const errorMessage = page.locator('text=Please provide a prompt');
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
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('Read the contents of README.md');

    // Try to select a project if available
    const projectButton = page
      .locator('button:has-text("No Project")')
      .or(page.locator('button').filter({ hasText: /Project/ }));
    if (await projectButton.isVisible()) {
      await projectButton.click();

      // Select first available project if dropdown appears
      const projectOption = page.locator('text=/Project/').first();
      if (await projectOption.isVisible()) {
        await projectOption.click();
      }
    }

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

    // Verify the prompt appears
    await expect(page.locator('text=Read the contents of README.md')).toBeVisible();
  });
});
