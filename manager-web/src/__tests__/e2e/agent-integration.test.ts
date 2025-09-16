import { expect, test } from './setup';

test.describe('Agent Integration', () => {
  test('should process file listing request and display results', async ({ page }) => {
    // Navigate to the dashboard
    await page.goto('/');

    // Wait for the page to load
    await page.waitForTimeout(1000); // Give time for components to render
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt for file listing
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
    await page.waitForTimeout(1000); // Give time for components to render
    await page.waitForSelector('h3:has-text("What would you like to Work on?")');

    // Fill in the prompt for file reading
    const promptTextarea = page.locator('textarea#prompt');
    await promptTextarea.fill('Read the contents of README.md');

    // Tool is now hardcoded to llm-agent per issue #110 - no selection needed

    // Submit the form
    const submitButton = page.locator('button[type="submit"]:has-text("Start Work Session")');
    await submitButton.click();

    // Wait for navigation to work detail page
    await page.waitForURL(/\/work\/work-\d+/);

    // Wait for agent response (longer time for WebSocket messages)
    await page.waitForTimeout(12000);

    // Verify we're on the work detail page
    await expect(page.locator('h1:has-text("Work Details")')).toBeVisible();

    // Check for file content response
    const responseContent = page.locator('[class*="bg-black"], [class*="text-gray-100"]');
    await expect(responseContent).toBeVisible();

    // The response should contain some text content (LLM agent response)
    // Since LLM responses are not deterministic, just check that some response content exists
    const contentText = await responseContent.textContent();
    expect(contentText).toBeTruthy();
    expect(contentText!.length).toBeGreaterThan(5); // Should have some content
  });

  test('should show agent tool usage in work details', async ({ page }) => {
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

    // Verify tool name is displayed
    await expect(page.locator('text=llm-agent')).toBeVisible();

    // Verify work status indicators are present
    const statusElements = page.locator('[class*="bg-"]');
    await expect(statusElements.first()).toBeVisible();
  });
});
