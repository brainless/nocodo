// Test data generator stub
export const testDataGenerator = {
  generateProjectData: () => ({
    name: 'Test Project',
    path: '/test/path',
    description: 'Test project for CI'
  }),
  generateWorkData: (options: any) => ({
    project_id: options.project_id,
    title: 'Test Work',
    description: 'Test work session'
  }),
  generateLlmAgentSessionData: (options: any) => ({
    work_id: options.work_id,
    provider: 'anthropic',
    model: 'claude-3-5-sonnet-20241022'
  }),
  reset: () => {},
};