# Multi-Agent Architecture

nocodo is a development team of LLM agents that builds full-stack Rust + SolidJS software on behalf of the user.

## Agents

| Agent | Scope | Status |
|---|---|---| 
| DB Developer | SQLite schema design | Active (`schema_designer`) |
| Backend Developer | Actix-web handlers, migrations | Planned |
| Frontend Developer | SolidJS components | Planned |
| Project Manager | Epic/Task orchestration, fan-out | Planned |

## Communication Model

**Chats are private.** Each agent's conversation history (reasoning, tool calls, clarifications) is internal working memory. It is not shared across agents.

**Tasks/Epics are shared.** Decisions and deliverables are written to a shared task store. This is the only communication plane between agents.

```
User ──► PM Agent ──► creates Epic + Tasks ──► assigns to focused agents
User ──► DB Developer ──► does work ──► writes Task ──► PM triages
```

## Rules

1. **PM is the only agent that can fan-out** — create tasks assigned to other agents.
2. **PM owns task metadata** — title and description are set at creation by PM (or by the backend for direct-prompt flows). Focused agents never update task title or description.
3. **Focused agents own task status and outputs** — `update_task_status` is their only write to the task store. Schema ready, API contract defined, component done.
4. **PM has a `list_pending_review_tasks` tool** — used at session start to triage work created via direct-prompt paths.
5. **Tasks carry `source_prompt`** — the original user intent verbatim, so focused agents have full context without reading PM's chat.

## Sessions and Tasks

A **session** is the private working memory for one agent working on one task. Sessions are scoped to `(task_id, agent_type)`.

**A user's prompt creates a task and a session atomically, before the agent runs.** The prompt is the task — `source_prompt` is set to the verbatim user text. The agent's first act is refining the task title and description, then doing the work.

This means the task list *is* the chat list. The UI entry point is always a task; clicking it opens its chat drawer. There is no separate "session" concept visible to the user.

```
agent_chat_session
  id, project_id, agent_type, task_id (NOT NULL), created_at

agent_chat_message  (unchanged)
  id, session_id, ...
```

`get_or_create_session(project_id, agent_type)` is removed. Replaced by `create_task_session(task_id, agent_type)` — always creates a new session, one per task per agent.

PM can answer "which session worked on task #5?" via `task_id` on the session. It cannot read the messages.

## Epic/Task Storage

### Rust Types (`agents/src/storage/mod.rs`)

```rust
pub struct Epic {
    pub id: Option<i64>,
    pub project_id: i64,
    pub title: String,
    pub description: String,
    pub source_prompt: String,       // original user intent verbatim
    pub status: EpicStatus,
    pub created_by_agent: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub enum EpicStatus { Open, InProgress, Done }

pub struct Task {
    pub id: Option<i64>,
    pub project_id: i64,
    pub epic_id: Option<i64>,        // null for tasks created via direct prompt
    pub title: String,
    pub description: String,
    pub source_prompt: String,       // original user intent verbatim
    pub assigned_to_agent: String,   // AgentType::as_str()
    pub status: TaskStatus,
    pub depends_on_task_id: Option<i64>,
    pub created_by_agent: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub enum TaskStatus { Open, InProgress, Review, Done, Blocked }
```

### Trait (`agents/src/storage/mod.rs`)

```rust
#[async_trait]
pub trait TaskStorage: Send + Sync {
    async fn create_epic(&self, epic: Epic) -> Result<i64, AgentError>;
    async fn update_epic_status(&self, id: i64, status: EpicStatus) -> Result<(), AgentError>;
    async fn get_epic(&self, id: i64) -> Result<Option<Epic>, AgentError>;
    async fn list_epics(&self, project_id: i64) -> Result<Vec<Epic>, AgentError>;

    async fn create_task(&self, task: Task) -> Result<i64, AgentError>;
    async fn update_task_status(&self, id: i64, status: TaskStatus) -> Result<(), AgentError>;
    async fn get_task(&self, id: i64) -> Result<Option<Task>, AgentError>;
    // Used by focused agents to find their work
    async fn list_tasks_for_agent(&self, project_id: i64, agent: &str) -> Result<Vec<Task>, AgentError>;
    // Used by PM at session start to triage direct-prompt tasks
    async fn list_pending_review_tasks(&self, project_id: i64) -> Result<Vec<Task>, AgentError>;
}
```

### SQLite Schema (new migration)

```sql
CREATE TABLE epic (
    id              INTEGER PRIMARY KEY,
    project_id      INTEGER NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT NOT NULL,
    source_prompt   TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'open',
    created_by_agent TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE task (
    id                  INTEGER PRIMARY KEY,
    project_id          INTEGER NOT NULL,
    epic_id             INTEGER REFERENCES epic(id),
    title               TEXT NOT NULL,
    description         TEXT NOT NULL,
    source_prompt       TEXT NOT NULL,
    assigned_to_agent   TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'open',
    depends_on_task_id  INTEGER REFERENCES task(id),
    created_by_agent    TEXT NOT NULL,
    created_at          INTEGER NOT NULL,
    updated_at          INTEGER NOT NULL
);

-- agent_chat_session: task_id is NOT NULL (clean break, product not launched)
-- Remove task_id column from existing migration; rewrite session table as:
CREATE TABLE agent_chat_session (
    id          INTEGER PRIMARY KEY,
    project_id  INTEGER NOT NULL,
    agent_type  TEXT NOT NULL,
    task_id     INTEGER NOT NULL REFERENCES task(id),
    created_at  INTEGER NOT NULL
);
```

### AgentStorage changes

`get_or_create_session(project_id, agent_type)` is removed. Replaced by:

```rust
async fn create_task_session(
    &self,
    project_id: i64,
    task_id: i64,
    agent_type: &str,
) -> Result<Session, AgentError>;
```

Always creates a new session — no "get or create". One session per task per agent.

## Rollout Plan

1. **Epic/Task storage** — SQLite migration, Rust types, `TaskStorage` trait + `SqliteTaskStorage` impl, `AgentStorage::get_or_create_task_session`
2. **DB Developer tools** — `create_task` / `update_task_status` tool calls; switch to task-scoped sessions; test end-to-end
3. **PM Agent** — new agent with `create_epic`, `create_task`, `assign_task`, `list_pending_review_tasks`; test PM → DB Developer hand-off
4. **Backend Developer Agent** — add agent; test DB → Backend cross-agent task dependency
5. **Frontend Developer Agent** — add agent; test full-stack Epic flow
