// Test API client stub
export const testApiClient = {
  createProject: async (data: any) => ({ id: 'test-project-id' }),
  createWork: async (data: any) => ({ work: { id: 'test-work-id' } }),
  createLlmAgentSession: async (data: any) => ({
    session: {
      id: 'test-session-id',
      work_id: data.work_id,
      provider: data.provider,
      model: data.model,
      system_prompt: data.system_prompt || 'Default system prompt'
    }
  }),
  addMessage: async (data: any) => ({ id: 'test-message-id' }),
  deleteProject: async (id: string) => {},
  cleanup: async () => {},
};