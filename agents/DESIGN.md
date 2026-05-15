# Multi-Agent Design: User Chats, PM/PO/EM Roles, Task State Machine, and Comments

## Summary

This document captures the design for:

1. New agent roles in addition to existing specialists: Product Owner (PO) and Engineering Manager (EM).
2. Replacing the agent-centric chat drawer with a single user-facing chat session.
3. PM and PO both participating in user chat from day one ("both agents always speak").
4. A task state machine: PM creates tasks as `draft`; PO transitions them to the appropriate next state; EM gates `ready` for tasks that need technical shaping; specialists work only `ready` tasks.
5. Adding a `user` table to support guest users now and email/password auth later.
6. First-class comments on Epics and Tasks with role-based access; comments are how specialists participate.

Repository scope reviewed for this design:

- `backend/`
- `agents/`
- `admin-gui/`
- `shared-types/`
- `DEVELOP.md`


## Current State (As-Is)

### Chat and Task Coupling

- Chat sessions are task-scoped in `agent_chat_session` (`task_id NOT NULL` since migration V12).
- User messages enter through agent chat endpoints (`/api/agents/{agent}/chat`) and may create tasks directly when `task_id` is absent.
- `admin-gui` chat drawer is agent-centric: PM, DB Engineer, Backend Engineer, Frontend Engineer listed as peer contacts; user picks any agent and chats.

### Roles and Assignment

- Active agents: `project_manager`, `db_engineer`, `ui_designer`, `backend_engineer`, `frontend_engineer`.
- Dispatcher routes tasks by `assigned_to_agent` string. PM is not auto-dispatched; it runs from HTTP chat.
- PM prompt currently restricts assignment to `db_engineer`.

### Epics and Tasks

- `epic`: unassigned; has `created_by_agent`; no comment surface.
- `task`: has `assigned_to_agent`; `status` is a free-form string (`pending`, `in_progress`, `done`, etc.).
- No first-class state machine; no `draft`/`ready` distinction.

### Users and Identity

- No `user` table. Authorship is implicit (HTTP request = "the user").


## Target State (To-Be)

### 1) Agent Roles

| Role | Scope | User-facing? |
|------|-------|--------------|
| `project_manager` (PM) | Project orchestration. Decides when intake has enough clarity. Calls `finalize_session` to atomically emit artifacts + complete the session in one step. (Future) edits existing epics/tasks. | Yes — in user chat |
| `product_owner` (PO) | User proxy. Validates user representation in artifacts. Transitions every task out of `draft`. Owns acceptance criteria and prioritization signals. | Yes — in user chat |
| `engineering_manager` (EM) | Technical shaping. Gates `needs_technical_shaping → ready` for tasks that need it. May read codebase. | No initially; will join user chat later. |
| `db_engineer`, `backend_engineer`, `frontend_engineer`, `ui_designer` | Execute `ready` tasks assigned to them. | No — interaction via task comments only. |

### 2) Single User Chat Surface

The existing chat drawer (PM/DB/BE/FE peer contacts) is removed.

There is one chat UX per user-facing session. PM and PO both observe and may respond to every user message. EM (and other agents) may be brought into chat in future without schema changes.

Persistent tables:

- `user_chat_session`
- `user_chat_message`

Agent-internal chat is preserved for task execution (`agent_chat_session` / `agent_chat_message`) — used by the dispatcher and by specialists during task work, not exposed as user entry points.

Technical conversation about a specific artifact happens on **task/epic comments**, not in chat. This is the principle: chat is for product conversation with non-technical roles; comments are for execution discussion anchored to artifacts.

### 3) User Chat Access Policy

- PM, PO: read/write to user chat sessions/messages from day one.
- EM: future.
- Specialists (DB/BE/FE/UI): no direct access. Reached via task comments.

### 4) Session Lifecycle

- A new user chat starts a new `user_chat_session`.
- While the session is `open`, the user may append messages and PM/PO continue to respond.
- PM decides when intake has enough clarity, creates the epic + task(s) (in `draft` state), and marks the session `completed`. This must be atomic (see Multi-Agent Chat Mechanics).
- Completion is terminal for now (no reopen).
- (Future) PM may edit existing epics/tasks from a follow-up session rather than always creating new artifacts.

### 5) Task Link to Intake Session

Tasks include `source_session_id` referencing `user_chat_session` — the audit trail from "user said X" → "task Y was created."

### 6) Task State Machine

States (canonical):

- `draft` — created by PM. Not yet validated by PO.
- `needs_technical_shaping` — PO validated; EM must shape before specialists can work it. Entered only for agent types whose policy requires shaping.
- `ready` — ready for the assigned specialist.
- `in_progress` — specialist is working.
- `done` — completed.
- `blocked` — (future) any role can flag.

Transitions:

| Transition | Owner agent | Tool |
|-----------|-------------|------|
| `(none)` → `draft` | PM | `finalize_session` |
| `draft` → `needs_technical_shaping` \| `ready` | PO | `validate_task` (calls `initial_state_for(agent_type)`) |
| `needs_technical_shaping` → `ready` | EM | `mark_task_ready` |
| `ready` → `in_progress` | specialist (auto on dispatch) | dispatcher |
| `in_progress` → `done` | specialist | `complete_task` |

The function `initial_state_for(agent_type) -> TaskState` is **a single chokepoint in code**:

```rust
pub fn initial_state_for(agent_type: AgentType) -> TaskState {
    match agent_type {
        AgentType::BackendEngineer | AgentType::FrontendEngineer => TaskState::NeedsTechnicalShaping,
        AgentType::DbEngineer | AgentType::UiDesigner => TaskState::Ready,
        _ => TaskState::Ready,
    }
}
```

Changing the policy = editing this function. No config, no runtime flag, no LLM-resolved policy. The engineering invariant ("backend tasks must pass EM") is enforced in Rust, not hoped for in a prompt.

The dispatcher refuses to hand a task to a specialist unless its state is `ready`.

Reuse the existing `task.status` column for these state values (migration maps current ad-hoc values to the canonical set).

### 7) Comments on Epics and Tasks

- `epic_comment`, `task_comment` tables (see Data Model).
- **Read**: users, PM, PO; assigned specialist on their own task.
- **Write**: same as read; specialists may comment only on their assigned task.
- Comments can ship on a track independent of the chat split (no migration dependency).
- Comments are how specialists participate in conversation about work.


## Multi-Agent Chat Mechanics

The most novel piece of this design.

### Turn-Taking Model: "Both Agents Always Speak"

For each user message in an open session:

1. Backend fires PM and PO **concurrently**.
2. Each agent decides whether to respond. **Empty/no-comment responses are allowed and not stored** — otherwise the chat fills with PO saying "no comment".
3. Each non-empty response is stored as its own turn with `agent_type` and a unique `turn_id`.

Rationale: simplest viable model. Doubles LLM cost per user turn but avoids building a "should I respond" classifier (LLMs do this poorly). The storage shape doesn't change if we later switch strategies — reversible.

### Response Shape (shared-types)

One user message produces N agent turns. The long-poll endpoint returns:

```rust
struct UserChatResponse {
  turns: Vec<AgentTurn>,
  all_done: bool,
}

struct AgentTurn {
  turn_id: i64,
  agent_type: AgentType,
  content: String,
  created_at: DateTime<Utc>,
}
```

Frontend renders each turn as its own bubble with the agent's identity visible. UI orders turns by `created_at`; PM and PO may complete out of order.

### Storage

- `user_chat_message` has `author_type` (`user`|`agent`|`system`), `author_user_id` (when user), `agent_type` (when agent), `turn_id` (groups multi-message turns from one agent), `content`, `created_at`.
- Multiple agent rows with different `agent_type` per user message is expected and normal.

### Artifact-Creation Turn (PM)

When PM decides intake is sufficient, it calls a single `finalize_session` tool:

```rust
finalize_session(
  final_message: String,        // what PM says to the user
  epic: EpicDef { title, description },
  tasks: Vec<TaskDef { title, description, assigned_to_agent }>,
)
```

The backend implementation executes one SQLite transaction:

1. Insert PM's `user_chat_message`.
2. Insert the `epic`.
3. Insert each `task` with `source_session_id` and `status = draft`.
4. Set `user_chat_session.completed_at`.

Atomicity is structural — one tool call, one transaction. There is no buffering and no staging table. PM must formulate the complete set of artifacts in a single response, which is the right constraint: PM should have full clarity before finalizing.

The separate `create_epic`, `create_task`, and `complete_session` tools are not needed and are not implemented.

PO's `validate_task` runs **after** the session is complete, on each resulting task. PO has full session context available in chat history.

(Future variant: PO reviews PM's draft artifacts in-chat *before* commit. Not now.)

### Concurrency

PM and PO each maintain their own LLM context per session but read the same shared message history. Storage writes are independent.

### Cost and Latency

- Two LLM calls per user turn. Acceptable for beta.
- Both responses long-poll on the same parent `user_chat_message.id`; client renders as they arrive.


## Data Model Changes

### New Tables

#### `user`

```sql
user (
  id INTEGER PRIMARY KEY,
  display_name TEXT NOT NULL,
  email TEXT UNIQUE NULL,           -- null for guests
  password_hash TEXT NULL,          -- null for guests
  is_guest BOOLEAN NOT NULL DEFAULT 1,
  created_at, updated_at
)
```

Guests get a row; later "claimed" by setting `email + password_hash + is_guest=0`. **No `external_id` field** — all auth is internal, integer FKs throughout.

`display_name` is asked on first chat ("What should we call you?") and stored on the guest row.

#### `user_chat_session`

```sql
user_chat_session (
  id INTEGER PRIMARY KEY,
  project_id INTEGER NOT NULL REFERENCES project(id),
  created_by_user_id INTEGER NOT NULL REFERENCES user(id),
  status TEXT NOT NULL,              -- 'open' | 'completed'
  created_at, updated_at,
  completed_at NULLABLE
)
```

#### `user_chat_message`

```sql
user_chat_message (
  id INTEGER PRIMARY KEY,
  session_id INTEGER NOT NULL REFERENCES user_chat_session(id),
  author_type TEXT NOT NULL,         -- 'user' | 'agent' | 'system'
  author_user_id INTEGER NULL REFERENCES user(id),  -- when author_type='user'
  agent_type TEXT NULL,              -- when author_type='agent'
  turn_id INTEGER NULL,              -- groups a single agent's response
  content TEXT NOT NULL,
  created_at
)
```

#### `epic_comment`, `task_comment`

```sql
epic_comment (
  id INTEGER PRIMARY KEY,
  epic_id INTEGER NOT NULL REFERENCES epic(id),
  author_type TEXT NOT NULL,         -- 'user' | 'agent'
  author_user_id INTEGER NULL REFERENCES user(id),
  agent_type TEXT NULL,
  content TEXT NOT NULL,
  created_at, updated_at
)

task_comment (
  -- same shape with task_id FK
)
```

### Task Table Changes

Add:

- `source_session_id INTEGER NULL REFERENCES user_chat_session(id)`

Modify:

- `task.status` adopts the canonical state machine values (`draft`, `needs_technical_shaping`, `ready`, `in_progress`, `done`, `blocked`). A migration step maps existing rows.

### Indexes

- `user_chat_session(project_id, status, created_at)`
- `user_chat_message(session_id, created_at)`
- `task(status, assigned_to_agent)` — dispatcher hot path
- `epic_comment(epic_id, created_at)`, `task_comment(task_id, created_at)`


## API and Service Changes

### User Chat API

- `POST /api/user-chats` — create new session + first message. If no user identity in request, create a guest user (requires `display_name`).
- `GET /api/user-chats?project_id=...` — list sessions.
- `GET /api/user-chats/{session_id}/messages` — fetch history.
- `POST /api/user-chats/{session_id}/messages` — append while `open`; triggers concurrent PM + PO turns.
- `GET /api/user-chats/{session_id}/messages/{message_id}/response` — long-poll; returns `UserChatResponse` (see Multi-Agent Chat Mechanics).

PM-initiated session completion is a side effect of PM's tool calls, not a separate endpoint.

### Task State Transition API

Transitions are agent-internal (called via agent tools), not exposed as user-facing endpoints. The dispatcher reads `task.status` when deciding what to run.

### Comments API

- `GET /api/epics/{id}/comments`
- `POST /api/epics/{id}/comments`
- `GET /api/tasks/{id}/comments`
- `POST /api/tasks/{id}/comments`

Authorization per role.

### Existing Agent Chat API

- `agent_chat_session` / `agent_chat_message` tables stay. They become internal — used by the dispatcher and specialists for execution traces.
- `/api/agents/{agent}/chat` endpoints for PM and DB Engineer are removed once the user chat path is live (Phase 6).


## Authorization and Permissions

### User Chats

- PM, PO: read/write and complete sessions.
- EM, specialists: denied initially. EM joins later via the same mechanism (additive permission).
- Users: read/write their own sessions.

### Comments

- Users, PM, PO: read/write all epics/tasks within their project.
- Assigned specialist: read/write on assigned task only.
- Unassigned specialists: no access.

### State Transitions

Each transition is gated by which agent owns the corresponding tool. Access is granted by code (importing the helper into that agent's `tools.rs`), not by runtime config. Revocation = removing the import.


## agents/ Crate Impact

### Storage Traits

Add:

- `UserStorage` — user creation (guest), display_name updates.
- `UserChatStorage` — sessions, messages, completion (atomic finalize transaction).
- `CommentStorage` — epic/task comments.

Extend:

- `TaskStorage` — read/write canonical `status` values, `source_session_id`, state-transition methods, `(future) update_task`.

### AgentType Registry

Extend `agents/src/storage/mod.rs:11-17`:

- `ProductOwner` → `"product_owner"`
- `EngineeringManager` → `"engineering_manager"`

### Per-Agent Tools

- **PM**: `finalize_session(final_message, epic, tasks)` — atomically commits artifacts + completes the session in one transaction.
- **PO**: `validate_task` (calls `initial_state_for` to pick next state), `comment_on_task`, `comment_on_epic`.
- **EM**: `mark_task_ready`, `comment_on_task`.
- **Specialist**: `complete_task`, `comment_on_assigned_task`.

### Prompts

- **PM**: clarify with user; recognize when intake has enough clarity; create artifacts; complete session. Avoid acting as user proxy (PO's job).
- **PO**: act as user proxy; validate user representation in PM's drafts; transition every task out of `draft`; comment when needed.
- **EM**: technical shaping (TBD content); read codebase as needed; transition `needs_technical_shaping → ready`.

### Initial State Policy

Single function (likely `agents/src/task_policy.rs`):

```rust
pub fn initial_state_for(agent_type: AgentType) -> TaskState { ... }
```

PO's `validate_task` tool reads this and writes the resulting status. To change policy: edit this function.


## admin-gui Impact

### Removed

- Chat drawer with PM/DB/BE/FE peer contacts.
- Per-agent chat pages.

### Added

- **User Chat UI**: single chat surface per session. Renders all turns (user, PM, PO) inline by `created_at`. Each agent turn shows the agent's identity.
- **First-chat display_name prompt**: if no user identity exists in localStorage, ask for `display_name` before the first message; create a guest user.
- **Sessions list**: project-scoped, status-filtered.
- **Epic/Task comments pane**: inline comments on each artifact view; permission-aware composer.

### Retained

- Task detail view: shows assigned agent, task `status`, `source_session_id` link back to the originating user chat.
- Per-task agent chat (`agent_chat_session`) remains visible to admins for execution debugging.


## Migration Plan

Additive, backward-compatible phases.

### Phase 1: Schema Expansion

- Add `user`, `user_chat_session`, `user_chat_message`, `epic_comment`, `task_comment`.
- Add `task.source_session_id`.
- Migrate `task.status` values to the canonical state set.
- Extend `AgentType` enum with `ProductOwner`, `EngineeringManager`.
- Add indexes per Data Model section.

### Phase 2: User Chat Backend

- Implement user chat endpoints.
- Implement `initial_state_for` policy function.
- Implement PM `finalize_session` tool (one transaction: message + epic + tasks + session completion).
- Implement PO tools (`validate_task`).
- "Both agents speak" turn-taking in the chat handler.
- Existing `/api/agents/{pm,db}/chat` paths remain operational.

### Phase 3: State Machine in Dispatcher

- Dispatcher picks up tasks only where `status='ready'`.
- Specialist `complete_task` tool writes `status='done'`.
- EM `mark_task_ready` tool for `needs_technical_shaping → ready` (EM prompt may be stubbed initially — admin manual transition acceptable until EM is fully implemented).

### Phase 4: UI Migration

- Replace chat drawer with single User Chat UI.
- Add guest user creation + display_name prompt.
- Add sessions list.
- Keep task chat view for execution visibility.

### Phase 5: Comments

- Comments API + UI.
- Role-based authorization.
- Independent track — can be developed in parallel with earlier phases.

### Phase 6: Decommission Legacy

- Remove `/api/agents/{agent}/chat` user-facing endpoints.
- Remove per-agent chat pages in admin-gui.
- Keep internal `agent_chat_session` for execution traces.

### Phase 7+ (Future)

- EM prompt + EM joins user chat.
- PM edits existing epics/tasks.
- Real auth (email/password); guest claim flow.
- Voting / multi-PM models for diverse thinking.


## Risks and Tradeoffs

1. **Multi-agent latency and cost** — every user message hits two LLMs. Acceptable for beta; revisit if cost or wait time becomes a problem.

2. **PM bottleneck** — single PM per session is a throughput cap. Mitigations (multiple PM instances, diverse models) are deferred.

3. **PO–PM disagreement** — PO validates *after* PM commits artifacts. If PO thinks artifacts are wrong, it must comment on the task, not undo. Acceptable now; tighten later (e.g., PO can transition back to `draft`) if friction emerges.

4. **Empty agent responses** — PO will often have nothing to add. Storage must silently discard empty turns; UI must not render them.

5. **State machine drift** — if specialists or the dispatcher mutate `status` outside their tools, the invariant breaks. Centralize state writes through helper functions with grep-able names.

6. **Guest identity loss** — clearing localStorage loses guest history. Document this; not blocking for beta.


## Open Questions

### Resolved

- Session reopening: terminal for now.
- Tasks per session: PM may create multiple.
- Epic creation: required (every intake produces an epic).
- PO rollout: full agent from day one, in user chat.
- EM scope: technical shaping; gates `ready`; will join chat later.
- Identity model: internal `user` table, integer FKs, guest → email/password upgrade path.
- Routing PM vs PO: no routing; both always speak.
- Initial-state policy: code function `initial_state_for(agent_type)`.
- Task draft semantics: PM creates as `draft`; PO transitions every task out.
- Attachments / rich text: plain text initially.
- Notifications: deferred.

### Still Open

- **PO disagreement with PM's draft** — comment-only now, or can PO transition back to `draft` and request a redo? (Lean: comment-only.)
- **EM blocking** — if EM refuses to mark `ready`, what state? Back to `draft` with a comment, or a new `blocked` state?
- **Display-name collisions** — two guests pick the same name. Allow? Disambiguate in UI by `user.id`?


## Proposed Next Steps

1. Define shared-types contracts:
   - `User`, `UserChatSession`, `UserChatMessage`, `AgentTurn`, `UserChatResponse`
   - `TaskState` enum (canonical values)
   - `EpicComment`, `TaskComment`
2. Write Phase 1 migrations (including `task.status` value migration).
3. Implement `initial_state_for` policy function and state-machine helpers in `agents/`.
4. Build user chat backend with "both agents speak" turn-taking; PM + PO prompts. PM uses `finalize_session` as its sole artifact-creation tool.
5. Replace admin-gui chat drawer with single chat UX + first-time display_name prompt.
6. Ship comments (independent track).
