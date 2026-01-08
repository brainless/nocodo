export interface AgentInfo {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
}

export interface SqliteAgentConfig {
  db_path: string;
}

export interface CodebaseAnalysisAgentConfig {
  path: string;
  max_depth?: number;
}

export type AgentConfig = { type: "sqlite" } & SqliteAgentConfig | { type: "codebase-analysis" } & CodebaseAnalysisAgentConfig;

export interface AgentExecutionRequest {
  user_prompt: string;
  config: AgentConfig;
}

export interface AgentsResponse {
  agents: AgentInfo[];
}

export interface AgentExecutionResponse {
  session_id: number;
  agent_name: string;
  status: string;
  result: string;
}

export interface ErrorResponse {
  error: string;
}