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

    // Select tool using custom dropdown
    const toolButton = page.locator('button[aria-haspopup="listbox"]').first();
    await toolButton.click();

    // Wait for dropdown options and select claude
    await page.locator('div[role="option"]:has-text("claude")').click();

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work Session")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/\d+/);

    // Wait for agent response with proper conditions
    await page.waitForSelector('[class*="bg-black"], [class*="text-gray-100"]', { timeout: 10000 });

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Check for file listing response
    // The response should contain file names or indicate successful file listing
    const responseContent = page.locator('[class*="bg-black"], [class*="text-gray-100"]');
    await expect(responseContent).toBeVisible();

    // Verify work completed successfully
    const completedBadge = page.locator('[class*="bg-green-100"]');
    const runningBadge = page.locator('[class*="bg-blue-100"]');

    // Either completed or still running (but should show some response)
    await expect(completedBadge.or(runningBadge)).toBeVisible();
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
    const toolButton = page.locator('button[aria-haspopup="listbox"]').first();
    await toolButton.click();

    // Wait for dropdown options and select claude
    await page.locator('div[role="option"]:has-text("claude")').click();

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/\d+/);

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
    const toolButton = page.locator('button[aria-haspopup="listbox"]').first();
    await toolButton.click();

    // Wait for dropdown options and select claude
    await page.locator('div[role="option"]:has-text("claude")').click();

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/\d+/);

    // Verify tool name is displayed
    await expect(page.locator('text=claude')).toBeVisible();

    // Verify work status indicators are present
    const statusElements = page.locator('[class*="bg-"]');
    await expect(statusElements.first()).toBeVisible();
  });
});
