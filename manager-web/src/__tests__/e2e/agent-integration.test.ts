import { expect, test } from './setup';

test.describe('Agent Integration', () => {
  test('should process file listing request and display results', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt for file listing
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('List all files in the root directory');

    // Select tool using custom dropdown if not already llm-agent
    // Find the tool button by its text content
    const toolButton = page
      .locator('button[aria-haspopup="listbox"]')
      .filter({ hasText: 'llm-agent' });
    const currentTool = await toolButton.textContent();

    if (currentTool?.trim() !== 'llm-agent') {
      await toolButton.click();
      // Wait for dropdown options and select llm-agent
      await page.locator('div[role="option"]:has-text("llm-agent")').click();
    }

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work Session")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/work-\d+/);

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Verify work was created with llm-agent tool
    await expect(page.locator('text=llm-agent')).toBeVisible();

    // Check for work status (may be running or completed)
    const statusElement = page.locator('[class*="bg-"]');
    await expect(statusElement.first()).toBeVisible();
  });

  test('should handle file reading request', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt for file reading
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('Read the contents of README.md');

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

    // Wait for agent response
    await page.waitForTimeout(5000);

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Check for file content response
    const responseContent = page.locator('[class*="bg-black"], [class*="text-gray-100"]');
    await expect(responseContent).toBeVisible();

    // The response should contain some text content (README content)
    await expect(page.locator('text=README.md')).toBeVisible();
  });

  test('should show agent tool usage in work details', async ({ page }) => {
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

    // Verify tool name is displayed
    await expect(page.locator('text=llm-agent')).toBeVisible();

    // Verify work status indicators are present
    const statusElements = page.locator('[class*="bg-"]');
    await expect(statusElements.first()).toBeVisible();
  });
});
