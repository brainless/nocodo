/**
 * Agent information for the agents list
 */
export type AgentInfo = { id: string, name: string, description: string, enabled: boolean, };


/**
 * Configuration for SQLite analysis agent
 */
export type SqliteAgentConfig = { db_path: string, };


/**
 * Configuration for codebase analysis agent
 */
export type CodebaseAnalysisAgentConfig = { path: string, max_depth: number | null, };


export type AgentConfig = { "type": "sqlite" } & SqliteAgentConfig | { "type": "codebase-analysis" } & CodebaseAnalysisAgentConfig;


/**
 * Generic agent execution request with type-safe config
 */
export type AgentExecutionRequest = { user_prompt: string, config: AgentConfig, };


/**
 * Response containing list of available agents
 */
export type AgentsResponse = { agents: Array<AgentInfo>, };


export type AgentExecutionResponse = { session_id: bigint, agent_name: string, status: string, result: string, };


export type ErrorResponse = { error: string, };
