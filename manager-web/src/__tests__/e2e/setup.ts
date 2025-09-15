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
    tool_name: 'llm-agent',
    status: 'running',
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

    // Mock individual work fetch API
    await page.route('**/api/work/work-123', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          work: mockWorkResponse.work,
          messages: [
            {
              id: 'message-123',
              content: 'List all files in the root directory',
              content_type: 'text',
              author_type: 'user',
              author_id: null,
              created_at: '2024-01-01T00:00:00Z',
            },
          ],
          total_messages: 1,
        }),
      });
    });

    // Mock individual session fetch API
    await page.route('**/api/work/*/sessions/*', async route => {
      await route.fulfill({
        status: 200,
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
      const MockWebSocket = class extends EventTarget {
        constructor(_url: string) {
          super();
          // Simulate successful connection immediately
          setTimeout(() => {
            this.dispatchEvent(new Event('open'));
          }, 10);
        }

        send() {
          // Mock sending messages
          setTimeout(() => {
            // Simulate receiving LLM agent chunks
            const mockResponse = 'Mock agent response: Files listed successfully';
            for (let i = 0; i < mockResponse.length; i++) {
              setTimeout(
                () => {
                  this.dispatchEvent(
                    new MessageEvent('message', {
                      data: JSON.stringify({
                        type: 'LlmAgentChunk',
                        payload: {
                          session_id: 'session-123',
                          content: mockResponse[i],
                        },
                      }),
                    })
                  );
                },
                2000 + i * 50
              ); // Stagger the chunks
            }
          }, 2000);
        }

        close() {
          this.dispatchEvent(new Event('close'));
        }
      };

      // Override the WebSocket constructor
      window.WebSocket = MockWebSocket as any;

      // Also mock the WebSocket client state for the provider
      // This ensures the status indicator shows connected
      const originalWebSocket = window.WebSocket;
      window.WebSocket = function(url: string) {
        const ws = new originalWebSocket(url);
        // Force the readyState to OPEN immediately
        Object.defineProperty(ws, 'readyState', {
          value: WebSocket.OPEN,
          writable: false
        });
        return ws;
      } as any;
    });

    await use(page);
  },
});

export { expect } from '@playwright/test';
