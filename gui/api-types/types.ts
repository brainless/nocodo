/**
 * Agent information for the agents list
 */
export type AgentInfo = {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
};

/**
 * Configuration for SQLite analysis agent
 */
export type SqliteAgentConfig = { db_path: string };

/**
 * Configuration for codebase analysis agent
 */
export type CodebaseAnalysisAgentConfig = {
  path: string;
  max_depth: number | null;
};

export type AgentConfig =
  | ({ type: 'sqlite' } & SqliteAgentConfig)
  | ({ type: 'codebase-analysis' } & CodebaseAnalysisAgentConfig);

/**
 * Generic agent execution request with type-safe config
 */
export type AgentExecutionRequest = {
  user_prompt: string;
  config: AgentConfig;
};

/**
 * Response containing list of available agents
 */
export type AgentsResponse = { agents: Array<AgentInfo> };

export type AgentExecutionResponse = {
  session_id: bigint;
  agent_name: string;
  status: string;
  result: string;
};

export type ErrorResponse = { error: string };

/**
 * API key configuration for the settings page
 */
export type ApiKeyConfig = {
  name: string;
  key: string | null;
  is_configured: boolean;
};

/**
 * Settings response containing API keys and configuration info
 */
export type SettingsResponse = {
  config_file_path: string;
  api_keys: Array<ApiKeyConfig>;
  projects_default_path: string | null;
};

/**
 * Request for updating API keys
 */
export type UpdateApiKeysRequest = {
  xai_api_key: string | null;
  openai_api_key: string | null;
  anthropic_api_key: string | null;
  zai_api_key: string | null;
  zai_coding_plan: boolean | null;
};
