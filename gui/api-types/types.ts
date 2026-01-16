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

/**
 * Configuration for Tesseract OCR agent
 */
export type TesseractAgentConfig = { image_path: string };

/**
 * Configuration for Structured JSON agent
 */
export type StructuredJsonAgentConfig = {
  type_names: Array<string>;
  domain_description: string;
};

export type AgentConfig =
  | ({ type: 'sqlite' } & SqliteAgentConfig)
  | ({ type: 'codebase-analysis' } & CodebaseAnalysisAgentConfig)
  | ({ type: 'tesseract' } & TesseractAgentConfig)
  | ({ type: 'structured-json' } & StructuredJsonAgentConfig);

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

export type SessionMessage = {
  role: string;
  content: string;
  created_at: bigint;
};

export type SessionToolCall = {
  tool_name: string;
  request: any;
  response: any;
  status: string;
  execution_time_ms: bigint | null;
};

export type SessionResponse = {
  id: bigint;
  agent_name: string;
  provider: string;
  model: string;
  system_prompt: string | null;
  user_prompt: string;
  config: any;
  status: string;
  result: string | null;
  messages: Array<SessionMessage>;
  tool_calls: Array<SessionToolCall>;
};

export type SessionListItem = {
  id: bigint;
  agent_name: string;
  user_prompt: string;
  started_at: bigint;
};

export type SessionListResponse = { sessions: Array<SessionListItem> };

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

/**
 * Project entity for project management
 */
export type Project = {
  id: number;
  name: string;
  description: string;
  created_at: bigint;
};

/**
 * Workflow entity
 */
export type Workflow = {
  id: number;
  project_id: number;
  name: string;
  parent_workflow_id: number | null;
  branch_condition: string | null;
  created_at: bigint;
};

/**
 * Workflow step entity
 */
export type WorkflowStep = {
  id: number;
  workflow_id: number;
  step_number: number;
  description: string;
  created_at: bigint;
};

/**
 * Response containing a single workflow with its steps
 */
export type WorkflowWithSteps = {
  workflow: Workflow;
  steps: Array<WorkflowStep>;
};

/**
 * Response for saving workflow
 */
export type SaveWorkflowRequest = { workflow: Array<WorkflowStepData> };

/**
 * Workflow step data for saving
 */
export type WorkflowStepData = {
  id: number;
  step_number: number;
  description: string;
};

/**
 * Ask the user a list of questions to gather information or confirm actions
 */
export type AskUserRequest = {
  /**
   * The main prompt or context for the questions
   */
  prompt: string;
  /**
   * List of questions to ask the user
   */
  questions: Array<UserQuestion>;
  /**
   * Whether the user responses are required (true) or optional (false)
   */
  required: boolean | null;
  /**
   * Optional timeout in seconds for user response
   */
  timeout_secs: bigint | null;
};

/**
 * Response from the ask_user tool containing user answers
 */
export type AskUserResponse = {
  /**
   * Whether the user responded to all required questions
   */
  completed: boolean;
  /**
   * User's responses to each question
   */
  responses: Array<UserQuestionResponse>;
  /**
   * Any error or status message
   */
  message: string;
  /**
   * How long the user took to respond (in seconds)
   */
  response_time_secs: number | null;
};

/**
 * Individual question to ask the user
 */
export type UserQuestion = {
  /**
   * Unique identifier for this question
   */
  id: string;
  /**
   * The question text to display to the user
   */
  question: string;
  /**
   * Type of response expected
   */
  type: QuestionType;
  /**
   * Default value if user doesn't provide one
   */
  default: string | null;
  /**
   * List of possible options for multiple choice or select questions
   */
  options: Array<string> | null;
  /**
   * Additional description or help text for the question
   */
  description: string | null;
  /**
   * Validation rules for the response
   */
  validation: QuestionValidation | null;
};

/**
 * Individual user response to a question
 */
export type UserQuestionResponse = {
  /**
   * ID of the question being answered
   */
  question_id: string;
  /**
   * The user's answer
   */
  answer: string;
  /**
   * Whether the response is valid according to validation rules
   */
  valid: boolean;
  /**
   * Validation error message if response is invalid
   */
  validation_error: string | null;
};

export type QuestionType = 'text';

/**
 * Validation rules for question responses
 */
export type QuestionValidation = {
  /**
   * Minimum length for text responses
   */
  min_length: number | null;
  /**
   * Maximum length for text responses
   */
  max_length: number | null;
  /**
   * Minimum value for numeric responses
   */
  min_value: number | null;
  /**
   * Maximum value for numeric responses
   */
  max_value: number | null;
  /**
   * Regular expression pattern for text validation
   */
  pattern: string | null;
  /**
   * Custom validation error message
   */
  error_message: string | null;
};
