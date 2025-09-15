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

    // Mock individual work fetch API (for any work ID)
    await page.route(/\/api\/work\/work-\d+/, async route => {
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
            {
              id: 'message-tool-1',
              content: 'Executing list_files tool to get directory contents...',
              content_type: 'tool_execution',
              author_type: 'system',
              author_id: null,
              created_at: '2024-01-01T00:00:01Z',
            },
            {
              id: 'message-llm-1',
              content: 'I found 4 files in the root directory: README.md, src/, package.json, and Cargo.toml.',
              content_type: 'llm_response',
              author_type: 'assistant',
              author_id: null,
              created_at: '2024-01-01T00:00:02Z',
            }
          ],
          total_messages: 3,
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

    // Mock tool execution API endpoints
    await page.route('**/api/tools/execute', async route => {
      const requestData = route.request().postDataJSON();

      if (requestData.tool_name === 'list_files') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            tool_name: 'list_files',
            status: 'completed',
            result: mockFileListResponse,
            execution_time: 1500
          }),
        });
      } else if (requestData.tool_name === 'read_file') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            tool_name: 'read_file',
            status: 'completed',
            result: mockFileContentResponse,
            execution_time: 800
          }),
        });
      } else {
        await route.fulfill({
          status: 400,
          contentType: 'application/json',
          body: JSON.stringify({
            error: 'Unknown tool',
            message: `Tool '${requestData.tool_name}' is not supported`
          }),
        });
      }
    });

    // Mock LLM agent session API with tool call simulation
    await page.route('**/api/work/*/sessions/*/execute', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          session_id: 'session-123',
          status: 'completed',
          tool_calls: [
            {
              id: 'tool_call_1',
              tool_name: 'list_files',
              parameters: { path: '.' },
              result: mockFileListResponse,
              executed_at: new Date().toISOString()
            }
          ],
          response: 'I found 4 files in the root directory: README.md, src/, package.json, and Cargo.toml.'
        }),
      });
    });

    // Mock WebSocket connection with enhanced tool execution simulation
    await page.addInitScript(() => {
      // Mock WebSocket for testing
      window.WebSocket = class extends EventTarget {
        private messageQueue: string[] = [];
        private isConnected = false;

        constructor(_url: string) {
          super();
          // Simulate successful connection
          setTimeout(() => {
            this.isConnected = true;
            this.dispatchEvent(new Event('open'));
          }, 100);
        }

        send(data: string) {
          if (!this.isConnected) return;

          try {
            const message = JSON.parse(data);

            // Simulate tool execution workflow
            if (message.type === 'start_work' && message.tool_name === 'llm-agent') {
              this.simulateToolExecution(message.prompt);
            }
          } catch (error) {
            console.warn('Error parsing WebSocket message:', error);
          }
        }

        private simulateToolExecution(prompt: string) {
          let events: any[] = [];

          // Determine what type of tool execution to simulate based on the prompt
          if (prompt.toLowerCase().includes('list') && prompt.toLowerCase().includes('file')) {
            // Simulate list_files tool execution
            events = [
              // Initial processing
              { delay: 500, type: 'LlmAgentChunk', content: 'Analyzing your request...' },
              { delay: 1000, type: 'LlmAgentChunk', content: 'I need to list files in the directory.' },
              { delay: 1500, type: 'LlmAgentChunk', content: 'Executing list_files tool...' },

              // Tool execution start
              { delay: 2000, type: 'AiSessionStatusChanged', session_id: 'session-123', status: 'running' },

              // Tool results
              { delay: 2500, type: 'LlmAgentChunk', content: 'Found 4 files:' },
              { delay: 2600, type: 'LlmAgentChunk', content: '- README.md (file)' },
              { delay: 2700, type: 'LlmAgentChunk', content: '- src (directory)' },
              { delay: 2800, type: 'LlmAgentChunk', content: '- package.json (file)' },
              { delay: 2900, type: 'LlmAgentChunk', content: '- Cargo.toml (file)' },

              // Follow-up response
              { delay: 3500, type: 'LlmAgentChunk', content: 'Here are the files in your root directory.' },
              { delay: 4000, type: 'LlmAgentChunk', content: 'Is there anything specific you\'d like me to help you with?' },

              // Completion
              { delay: 4500, type: 'AiSessionCompleted', session_id: 'session-123' },
            ];
          } else if (prompt.toLowerCase().includes('read') && prompt.toLowerCase().includes('readme')) {
            // Simulate read_file tool execution for README
            events = [
              // Initial processing
              { delay: 500, type: 'LlmAgentChunk', content: 'Analyzing your request...' },
              { delay: 1000, type: 'LlmAgentChunk', content: 'I need to read the README.md file.' },
              { delay: 1500, type: 'LlmAgentChunk', content: 'Executing read_file tool...' },

              // Tool execution start
              { delay: 2000, type: 'AiSessionStatusChanged', session_id: 'session-123', status: 'running' },

              // Tool results
              { delay: 2500, type: 'LlmAgentChunk', content: 'Reading file contents...' },
              { delay: 3000, type: 'LlmAgentChunk', content: '# Test README' },
              { delay: 3200, type: 'LlmAgentChunk', content: '' },
              { delay: 3400, type: 'LlmAgentChunk', content: 'This is a test file for E2E testing.' },

              // Follow-up response
              { delay: 4000, type: 'LlmAgentChunk', content: 'I\'ve read the README.md file for you.' },
              { delay: 4500, type: 'LlmAgentChunk', content: 'It contains test content for E2E testing.' },

              // Completion
              { delay: 5000, type: 'AiSessionCompleted', session_id: 'session-123' },
            ];
          } else if (prompt.toLowerCase().includes('list') && prompt.toLowerCase().includes('read')) {
            // Simulate multiple tool calls
            events = [
              // Initial processing
              { delay: 500, type: 'LlmAgentChunk', content: 'Analyzing your request...' },
              { delay: 1000, type: 'LlmAgentChunk', content: 'I need to perform multiple operations.' },
              { delay: 1500, type: 'LlmAgentChunk', content: 'First, executing list_files tool...' },

              // First tool execution
              { delay: 2000, type: 'AiSessionStatusChanged', session_id: 'session-123', status: 'running' },
              { delay: 2500, type: 'LlmAgentChunk', content: 'Found 4 files:' },
              { delay: 2600, type: 'LlmAgentChunk', content: '- README.md (file)' },
              { delay: 2700, type: 'LlmAgentChunk', content: '- src (directory)' },
              { delay: 2800, type: 'LlmAgentChunk', content: '- package.json (file)' },
              { delay: 2900, type: 'LlmAgentChunk', content: '- Cargo.toml (file)' },

              // Second tool execution
              { delay: 3500, type: 'LlmAgentChunk', content: 'Now reading README.md file...' },
              { delay: 4000, type: 'LlmAgentChunk', content: 'Executing read_file tool...' },
              { delay: 4500, type: 'LlmAgentChunk', content: '# Test README' },
              { delay: 4700, type: 'LlmAgentChunk', content: '' },
              { delay: 4900, type: 'LlmAgentChunk', content: 'This is a test file for E2E testing.' },

              // Follow-up response
              { delay: 5500, type: 'LlmAgentChunk', content: 'I\'ve completed both operations.' },
              { delay: 6000, type: 'LlmAgentChunk', content: 'The directory contains 4 files, and I\'ve read the README.md content.' },

              // Completion
              { delay: 6500, type: 'AiSessionCompleted', session_id: 'session-123' },
            ];
          } else if (prompt.toLowerCase().includes('nonexistent') || prompt.toLowerCase().includes('invalid')) {
            // Simulate tool execution error
            events = [
              // Initial processing
              { delay: 500, type: 'LlmAgentChunk', content: 'Analyzing your request...' },
              { delay: 1000, type: 'LlmAgentChunk', content: 'I need to access the specified path.' },
              { delay: 1500, type: 'LlmAgentChunk', content: 'Executing tool...' },

              // Tool execution start
              { delay: 2000, type: 'AiSessionStatusChanged', session_id: 'session-123', status: 'running' },

              // Error during execution
              { delay: 3000, type: 'LlmAgentChunk', content: 'Error: Path does not exist or is not accessible.' },
              { delay: 3500, type: 'LlmAgentChunk', content: 'The requested file or directory was not found.' },

              // Error completion
              { delay: 4000, type: 'AiSessionStatusChanged', session_id: 'session-123', status: 'failed' },
              { delay: 4500, type: 'AiSessionCompleted', session_id: 'session-123' },
            ];
          } else if (prompt.toLowerCase().includes('passwd') || prompt.toLowerCase().includes('system')) {
            // Simulate permission/access error
            events = [
              // Initial processing
              { delay: 500, type: 'LlmAgentChunk', content: 'Analyzing your request...' },
              { delay: 1000, type: 'LlmAgentChunk', content: 'I cannot access system files for security reasons.' },
              { delay: 1500, type: 'LlmAgentChunk', content: 'Access denied: Permission error.' },

              // Error completion
              { delay: 2000, type: 'AiSessionStatusChanged', session_id: 'session-123', status: 'failed' },
              { delay: 2500, type: 'AiSessionCompleted', session_id: 'session-123' },
            ];
          } else {
            // Default response for other prompts
            events = [
              { delay: 500, type: 'LlmAgentChunk', content: 'Processing your request...' },
              { delay: 1500, type: 'AiSessionStatusChanged', session_id: 'session-123', status: 'running' },
              { delay: 2500, type: 'LlmAgentChunk', content: 'Request processed successfully.' },
              { delay: 3500, type: 'AiSessionCompleted', session_id: 'session-123' },
            ];
          }

          events.forEach(event => {
            setTimeout(() => {
              if (event.type === 'LlmAgentChunk') {
                this.dispatchEvent(
                  new MessageEvent('message', {
                    data: JSON.stringify({
                      type: event.type,
                      payload: {
                        session_id: 'session-123',
                        content: event.content,
                      },
                    }),
                  })
                );
              } else if (event.type === 'AiSessionStatusChanged') {
                this.dispatchEvent(
                  new MessageEvent('message', {
                    data: JSON.stringify({
                      type: event.type,
                      payload: {
                        session_id: event.session_id,
                        status: event.status,
                      },
                    }),
                  })
                );
              } else if (event.type === 'AiSessionCompleted') {
                this.dispatchEvent(
                  new MessageEvent('message', {
                    data: JSON.stringify({
                      type: event.type,
                      payload: {
                        session_id: event.session_id,
                      },
                    }),
                  })
                );
              }
            }, event.delay);
          });
        }

        close() {
          this.isConnected = false;
          this.dispatchEvent(new Event('close'));
        }
      } as any;
    });

    await use(page);
  },
});

export { expect } from '@playwright/test';
