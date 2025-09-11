import { expect, test } from './setup';

test.describe('File Operations', () => {
  test('should successfully read and display file contents', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt for reading README
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('Read the contents of README.md');

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
    await page.waitForTimeout(8000); // Give more time for file reading

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Check that file content is displayed
    const responseContent = page.locator('[class*="bg-black"], [class*="text-gray-100"]');

    // Wait for content to appear
    await page.waitForSelector('[class*="bg-black"], [class*="text-gray-100"]', { timeout: 15000 });

    // Verify some content is present (should contain README text)
    const contentText = await responseContent.textContent();
    expect(contentText).toBeTruthy();
    expect(contentText!.length).toBeGreaterThan(10); // Should have substantial content
  });

  test('should handle file reading with different file types', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt for reading package.json
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('Read the contents of package.json');

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

    // Check for JSON content (should contain package.json structure)
    const responseContent = page.locator('[class*="bg-black"], [class*="text-gray-100"]');
    await page.waitForSelector('[class*="bg-black"], [class*="text-gray-100"]', { timeout: 15000 });

    // Verify JSON content is present
    const contentText = await responseContent.textContent();
    expect(contentText).toContain('nocodo-manager-web');
    expect(contentText).toContain('dependencies');
  });

  test('should handle file reading errors gracefully', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt for reading non-existent file
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('Read the contents of nonexistent-file.txt');

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

    // Check for error handling - should show error message or failed status
    const errorElements = page.locator('[class*="bg-red-100"], [class*="text-red"]');
    const failedBadge = page.locator('[class*="bg-red-100"]');

    // Either error message or failed status should be visible
    try {
      await expect(errorElements.or(failedBadge)).toBeVisible({ timeout: 5000 });
    } catch {
      // If no explicit error, check that work completed (might handle gracefully)
      const completedBadge = page.locator('[class*="bg-green-100"]');
      await expect(completedBadge).toBeVisible();
    }
  });
});
