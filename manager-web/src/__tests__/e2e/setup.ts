import { test as base } from '@playwright/test';

// Mock data for testing
export const mockProjects = [
  {
    id: 'project-1',
    name: 'Test Project 1',
    language: 'rust',
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  },
  {
    id: 'project-2',
    name: 'Test Project 2',
    language: 'typescript',
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  },
];

export const mockWorkResponse = {
  work: {
    id: 'work-123',
    title: 'Test Work',
    project_id: null,
    created_at: '2024-01-01T00:00:00Z',
    updated_at: '2024-01-01T00:00:00Z',
  },
};

export const mockMessageResponse = {
  message: {
    id: 'message-123',
    content: 'Test prompt',
    content_type: 'text',
    author_type: 'user',
    author_id: null,
    created_at: '2024-01-01T00:00:00Z',
  },
};

export const mockAiSessionResponse = {
  session: {
    id: 'session-123',
    work_id: 'work-123',
    message_id: 'message-123',
    tool_name: 'llm-agent',
    status: 'running',
    project_context: null,
    started_at: Math.floor(Date.now() / 1000),
    ended_at: null,
  },
};

export const mockFileListResponse = [
  { name: 'README.md', type: 'file', size: 1024 },
  { name: 'src', type: 'directory', size: 0 },
  { name: 'package.json', type: 'file', size: 512 },
  { name: 'Cargo.toml', type: 'file', size: 256 },
];

export const mockFileContentResponse = {
  content: '# Test README\n\nThis is a test README file for E2E testing.',
  encoding: 'utf-8',
};

// Extend the base test with mocking capabilities
export const test = base.extend({
  // Mock API responses
  page: async ({ page }, use) => {
    // Mock projects API
    await page.route('**/api/projects', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockProjects),
      });
    });

    // Mock work creation API
    await page.route('**/api/work', async route => {
      if (route.request().method() === 'POST') {
        await route.fulfill({
          status: 201,
          contentType: 'application/json',
          body: JSON.stringify(mockWorkResponse),
        });
      } else {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify([mockWorkResponse.work]),
        });
      }
    });

    // Mock message creation API
    await page.route('**/api/work/*/messages', async route => {
      await route.fulfill({
        status: 201,
        contentType: 'application/json',
        body: JSON.stringify(mockMessageResponse),
      });
    });

    // Mock AI session creation API
    await page.route('**/api/work/*/sessions', async route => {
      await route.fulfill({
        status: 201,
        contentType: 'application/json',
        body: JSON.stringify(mockAiSessionResponse),
      });
    });

    // Mock file listing API (for agent tool)
    await page.route('**/api/files/list**', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockFileListResponse),
      });
    });

    // Mock file reading API (for agent tool)
    await page.route('**/api/files/read**', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(mockFileContentResponse),
      });
    });

    // Mock WebSocket connection (simulate successful connection)
    await page.addInitScript(() => {
      // Mock WebSocket for testing
      window.WebSocket = class extends EventTarget {
        constructor(_url: string) {
          super();
          // Simulate successful connection
          setTimeout(() => {
            this.dispatchEvent(new Event('open'));
          }, 100);
        }

        send() {
          // Mock sending messages
          setTimeout(() => {
            // Simulate receiving a response
            this.dispatchEvent(
              new MessageEvent('message', {
                data: JSON.stringify({
                  type: 'work_update',
                  work_id: 'work-123',
                  status: 'completed',
                  output: 'Mock agent response: Files listed successfully',
                }),
              })
            );
          }, 2000);
        }

        close() {
          this.dispatchEvent(new Event('close'));
        }
      } as any;
    });

    await use(page);
  },
});

export { expect } from '@playwright/test';
