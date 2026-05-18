# Multi-Agent Design: Two-Phase User Chat (PO Intake → PM Planning), Task State Machine, and Comments

## Summary

This document captures the design for:

1. A two-phase user chat flow: Product Owner (PO) handles requirements intake alone, then hands off to Project Manager (PM) for planning.
2. Replacing the agent-centric chat drawer with a single user-facing chat session.
3. PO as the sole intake agent — PM is NOT present during requirements gathering.
4. PO calls `hand_off_to_pm` when intake is complete, which atomically closes the intake session, creates a new planning session seeded with a summary, and fires PM once.
5. PM runs only in the planning session, gathers additional detail if needed, then calls `finalize_session` to create epics/tasks.
6. A task state machine: PM creates tasks as `draft`; PO transitions them to the appropriate next state after PM finalizes.
7. Adding a `user` table to support guest users now and email/password auth later.
8. First-class comments on Epics and Tasks with role-based access; comments are how specialists participate.

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
| `product_owner` (PO) | Requirements intake. Listens to user, gathers requirements via text and structured questions. Calls `hand_off_to_pm` when enough clarity. Validates tasks after PM creates them. | Yes — sole agent in intake session |
| `project_manager` (PM) | Planning and artifact creation. Runs only in the planning session (created by PO handoff). Asks follow-up questions if needed. Calls `finalize_session` to atomically create epic + tasks. | Yes — in planning session only |
| `engineering_manager` (EM) | Technical shaping. Gates `needs_technical_shaping → ready` for tasks that need it. May read codebase. | No initially; will join user chat later. |
| `db_engineer`, `backend_engineer`, `frontend_engineer`, `ui_designer` | Execute `ready` tasks assigned to them. | No — interaction via task comments only. |

### 2) Two-Phase User Chat Surface

The existing chat drawer (PM/DB/BE/FE peer contacts) is removed.

There are two chat phases per user-facing initiative:

**Phase 1 — Intake session (PO only):** The user describes what they want to build. PO asks clarifying questions (text and structured widgets) to understand the business context, users, data, features, and constraints. PO does NOT create artifacts. When PO has enough clarity, it calls `hand_off_to_pm`.

**Phase 2 — Planning session (PM only):** Created automatically by the handoff. Seeded with PO's requirements summary. PM may ask follow-up questions. When PM has enough clarity, it calls `finalize_session` to create the epic and tasks atomically.

PM and PO are NEVER in the same session concurrently. The backend detects planning sessions by checking if the first message is from `product_owner` and routes accordingly (`run_pm_planning` vs `run_po_intake`).

Persistent tables:

- `user_chat_session` (with `handoff_session_id` linking intake → planning)
- `user_chat_message`

Agent-internal chat is preserved for task execution (`agent_chat_session` / `agent_chat_message`) — used by the dispatcher and by specialists during task work, not exposed as user entry points.

Technical conversation about a specific artifact happens on **task/epic comments**, not in chat. This is the principle: chat is for product conversation with non-technical roles; comments are for execution discussion anchored to artifacts.

### 3) User Chat Access Policy

- PO: read/write to intake sessions. Calls `hand_off_to_pm` to transition to planning.
- PM: read/write to planning sessions only. Calls `finalize_session` to create artifacts.
- EM: future.
- Specialists (DB/BE/FE/UI): no direct access. Reached via task comments.

### 4) Session Lifecycle

**Intake session:**
- A new user chat starts a new `user_chat_session` (intake).
- While the session is `open`, the user may append messages and PO continues to respond.
- PO gathers requirements via text and structured questions (`request_user_input`).
- When PO has enough clarity, it calls `hand_off_to_pm(final_message, summary)`.

**Handoff (atomic):**
1. PO's closing message is stored in the intake session.
2. A new planning session is created.
3. PO's requirements summary is seeded as the first message in the planning session (from `product_owner`).
4. `handoff_session_id` is set on the intake session, linking it to the planning session.
5. The intake session is marked `completed`.
6. PM is fired once in the background in the planning session.

**Planning session:**
- PM reads the PO summary + any prior context.
- PM may ask follow-up questions (text or structured).
- When PM has enough clarity, it calls `finalize_session` to create epic + tasks.
- After PM finalizes, PO validates all tasks (transitions out of `draft`).

Completion is terminal for now (no reopen).

### 5) Task Link to Session

Tasks include `source_session_id` referencing `user_chat_session` — the audit trail from "user said X" → "task Y was created." This points to the planning session where PM created the artifacts. The `handoff_session_id` on the planning session's parent (intake) session links back to the original user conversation.

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


## Two-Phase Chat Mechanics

The most novel piece of this design.

### Phase 1: PO Intake (Single Agent)

For each user message in an open intake session:

1. Backend fires **PO only** (`run_po_intake`).
2. PO decides whether to respond with text, structured questions, or handoff.
3. **Empty/no-comment responses are silently discarded** — stored as `PoSessionResult::Silent`.
4. PO has three possible outcomes per turn:
   - **Text response** (`PoSessionResult::Text`) — stored as a message with `agent_type = product_owner`.
   - **Structured questions** (`PoSessionResult::Questions`) — one or more `structured_question` messages stored; backend waits for user answers before firing PO again.
   - **Handoff** (`PoSessionResult::HandedOff`) — triggers the handoff flow (see below).

**Unanswered structured question guard:** If there are `structured_question` messages without matching `structured_response` messages, the backend does NOT fire PO. PO runs only after the user has answered all pending questions.

### Phase 2: PM Planning (Single Agent)

Triggered automatically when PO calls `hand_off_to_pm`:

1. Backend creates a new planning session and seeds it with PO's summary.
2. Backend fires **PM only** (`run_pm_planning`).
3. PM may respond with text, structured questions, or call `finalize_session`.
4. After PM finalizes (atomic: message + epic + tasks + session completion), PO validates all created tasks.

**Session routing:** The backend detects planning sessions by checking if the first message is from `product_owner`. If so, it delegates to `run_pm_planning` instead of `run_po_intake`. This ensures PO doesn't respond alongside PM in planning sessions.

### Handoff Flow

When PO calls `hand_off_to_pm(final_message, summary)`:

```rust
struct HandOffToPmParams {
    final_message: String,  // warm closing message to the user
    summary: String,        // structured requirements brief for PM
}
```

Backend executes:
1. Store PO's `final_message` in the intake session.
2. Create a new `user_chat_session` (planning session).
3. Seed planning session with `summary` as the first message (author: `product_owner`).
4. Set `handoff_session_id` on the intake session → links to planning session.
5. Mark intake session `completed`.
6. Fire PM in the planning session in the background.

The frontend detects `handoff_session_id` in the GET /messages response and navigates to the planning session URL.

### Response Shape

One user message produces one agent turn (PO in intake, PM in planning). The poll endpoint returns:

```rust
struct GetMessagesResponse {
    session_id: i64,
    messages: Vec<UserChatMessageRow>,
    handoff_session_id: Option<i64>,  // present when PO has handed off
}
```

Frontend renders messages by `created_at`. When `handoff_session_id` is present, the UI navigates to the planning session.

### Storage

- `user_chat_message` has `author_type` (`user`|`agent`|`system`), `author_user_id` (when user), `agent_type` (when agent), `turn_id` (nullable, not actively used in single-agent flow), `content_type` (`text`|`structured_question`|`structured_response`), `content` (plain string or JSON), `created_at`.
- Only one agent responds per session at a time — no concurrent multi-agent turns.

### Artifact-Creation Turn (PM, in Planning Session)

When PM decides planning is sufficient, it calls a single `finalize_session` tool:

```rust
finalize_session(
  final_message: String,        // what PM says to the user
  epic_title: String,
  epic_description: String,
  tasks: Vec<TaskDef { title, description, assigned_to_agent }>,
)
```

The backend implementation executes one SQLite transaction:
1. Insert PM's `user_chat_message`.
2. Insert the `epic`.
3. Insert each `task` with `source_session_id` and `status = draft`.
4. Set `user_chat_session.completed_at`.

Atomicity is structural — one tool call, one transaction. PM must formulate the complete set of artifacts in a single response.

**Post-finalization:** After PM commits, the backend immediately fires PO's `validate_tasks` on all newly created task IDs. PO transitions each task out of `draft` using `initial_state_for(agent_type)`.

### Concurrency

PO and PM each run in their own session. Only one agent fires per session at a time. Storage writes are independent. The handoff creates a new session and fires PM in the background — the user sees the handoff message and is redirected to the planning session.

### Cost and Latency

- One LLM call per user turn (PO during intake, PM during planning).
- Long-poll endpoint (`GET /api/user-chats/{session_id}/poll?after=X`) holds the connection until a new message arrives or 30s elapses.
- Per-session `Notify` map wakes waiters when agents store messages.
- Handoff adds one extra LLM call (PM fires once after handoff).


## Structured User Input

### Motivation

Free-text answers slow the user down and produce noisy LLM context. For questions where a finite set of choices covers the common cases (who are the users? what data needs tracking? which features are in scope?) we instead render interactive widgets and let the user click.

### `request_user_input` Tool

Available to PO (intake) and PM (planning). Parameters:

```rust
struct RequestUserInputParams {
    question: String,
    input_type: InputType,   // SingleChoice | MultipleChoice
    options: Vec<String>,    // 2–6 short labels
}
```

Defined in `agents/src/user_input_tool.rs` — shared, not agent-specific.

### Message Content Types

`user_chat_message` carries `(content_type, content)`:

| `content_type`        | Writer        | `content` payload                          |
|-----------------------|---------------|--------------------------------------------|
| `text`                | user or agent | plain string                               |
| `structured_question` | agent (PM/PO) | JSON `StructuredQuestion`                  |
| `structured_response` | user (widget) | JSON `StructuredResponse`                  |

Types live in `agents/src/storage/message_content.rs`. `MessageContent` is the Rust enum used throughout; DB and frontend see the raw columns.

### `StructuredQuestion` and Extensibility

```rust
struct StructuredQuestion {
    question: String,
    kind: QuestionKind,
}

enum QuestionKind {          // <-- extend here for new input types
    SingleChoice  { options: Vec<String> },
    MultipleChoice { options: Vec<String> },
    // Future: Rating { min: u8, max: u8 }, Scale, ...
}
```

`QuestionKind` is the extensibility point. `StructuredQuestion` and `StructuredResponse` stay stable.

### `StructuredResponse`

```rust
struct StructuredResponse {
    question_message_id: i64,   // FK to the matching structured_question row
    selected: Vec<String>,
}
```

`question_message_id` lets the UI mark the original widget as "answered" and lets agents correlate answers to questions when session history grows.

### LLM Context Representation

Structured messages are compiled to plain text before being sent to the LLM via `MessageContent::to_llm_text()`:

- `structured_question` → `"Who will enter data? (pick all that apply): Internal staff, Volunteers, Candidates"`
- `structured_response` → `"Selected: Internal staff, Volunteers"`

The LLM never sees raw JSON.

### Flow

**Intake phase:**
1. User sends a message → PO calls `request_user_input` (or responds with text).
2. Backend stores a `structured_question` row and returns; PO does not emit additional text.
3. Frontend renders the widget (radio or checkboxes).
4. User submits → frontend sends `POST /api/user-chats/{id}/messages` with `content_type: "structured_response"`.
5. Backend stores a `structured_response` row and fires PO again (only after all pending questions are answered).
6. PO reads the compiled answer and continues gathering requirements or calls `hand_off_to_pm`.

**Planning phase (after handoff):**
1. PM receives PO's summary in the planning session.
2. PM may call `request_user_input` for follow-up questions.
3. User answers → PM continues or calls `finalize_session`.
4. Backend atomically creates epic + tasks, then fires PO to validate tasks.


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
  handoff_session_id INTEGER NULL REFERENCES user_chat_session(id),  -- links intake → planning
  created_at, updated_at,
  completed_at NULLABLE
)
```

`handoff_session_id` is set when PO calls `hand_off_to_pm`, linking the intake session to the newly created planning session.

#### `user_chat_message`

```sql
user_chat_message (
  id INTEGER PRIMARY KEY,
  session_id INTEGER NOT NULL REFERENCES user_chat_session(id),
  author_type TEXT NOT NULL,         -- 'user' | 'agent' | 'system'
  author_user_id INTEGER NULL REFERENCES user(id),  -- when author_type='user'
  agent_type TEXT NULL,              -- when author_type='agent'
  turn_id INTEGER NULL,              -- groups a single agent's response
  content_type TEXT NOT NULL DEFAULT 'text',  -- 'text' | 'structured_question' | 'structured_response'
  content TEXT NOT NULL,             -- plain string for 'text'; JSON for structured variants
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

- `POST /api/user-chats` — create new intake session + first message. Creates a guest user if no identity exists (requires `display_name`). Fires PO in background.
- `GET /api/user-chats?project_id=...` — list sessions.
- `GET /api/user-chats/{session_id}/messages` — fetch history. Includes `handoff_session_id` if PO has handed off.
- `POST /api/user-chats/{session_id}/messages` — append while `open`; fires PO (intake) or PM (planning) based on session type.
- `GET /api/user-chats/{session_id}/poll?after=X` — long-poll; holds until a new message arrives or 30s elapses. Returns `GetMessagesResponse` with `handoff_session_id`.

PO-initiated handoff creates a planning session automatically. PM-initiated session completion (`finalize_session`) is a side effect of PM's tool call, not a separate endpoint.

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

- **PO**: `hand_off_to_pm(final_message, summary)` — closes intake session, creates planning session, fires PM. `request_user_input(question, input_type, options)` — poses structured choice questions during intake. `validate_tasks(task_ids)` — transitions all PM-created tasks out of `draft` using `initial_state_for`. `comment_on_task`, `comment_on_epic`.
- **PM**: `finalize_session(final_message, epic_title, epic_description, tasks)` — atomically commits artifacts + completes the planning session in one transaction. `request_user_input(question, input_type, options)` — poses structured choice questions during planning.
- **EM**: `mark_task_ready`, `comment_on_task`.
- **Specialist**: `complete_task`, `comment_on_assigned_task`.

The `request_user_input` tool is defined in `agents/src/user_input_tool.rs` and shared. It is not agent-specific.

### Prompts

- **PO (intake)**: warm, empathetic, non-technical. Gather requirements — business context, users, data, features, constraints. Use `request_user_input` for questions with clear choices. Call `hand_off_to_pm` when enough clarity. Never refer to internal roles or the handoff to the user.
- **PM (planning)**: receives PO's summary. Ask follow-up questions if needed. Create epic + tasks via `finalize_session`. Assign tasks to appropriate agents.
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

- **User Chat UI**: single chat surface per session. Renders messages inline by `created_at`. Agent messages show the agent's identity (PO or PM).
- **Structured question widgets**: when PO or PM emits a `structured_question` message, the UI renders radio buttons (single choice) or checkboxes (multiple choice) instead of a text input. The user submits their selection; it is stored as a `structured_response` and compiled to plain text for the LLM.
- **First-chat display_name prompt**: if no user identity exists in localStorage, ask for `display_name` before the first message; create a guest user.
- **Sessions list**: project-scoped, status-filtered.
- **Handoff navigation**: when `handoff_session_id` is present in the messages response, the UI automatically navigates to the planning session. Completed intake sessions can still be clicked to view the conversation.
- **Epic/Task comments pane**: inline comments on each artifact view; permission-aware composer.

### Retained

- Task detail view: shows assigned agent, task `status`, `source_session_id` link back to the originating user chat.
- Per-task agent chat (`agent_chat_session`) remains visible to admins for execution debugging.


## Migration Plan

Additive, backward-compatible phases.

### Phase 1: Schema Expansion ✅ DONE

- Add `user`, `user_chat_session`, `user_chat_message`, `epic_comment`, `task_comment`.
- Add `task.source_session_id`.
- Migrate `task.status` values to the canonical state set.
- Extend `AgentType` enum with `ProductOwner`, `EngineeringManager`.
- Add indexes per Data Model section.
- Add `user_chat_session.handoff_session_id` (V21).

### Phase 2: User Chat Backend ✅ DONE (Two-Phase)

- Implement user chat endpoints.
- Implement `initial_state_for` policy function.
- Implement PM `finalize_session` tool (one transaction: message + epic + tasks + session completion).
- Implement PO `hand_off_to_pm` tool — atomically closes intake, creates planning session, fires PM.
- Implement PO `validate_tasks` — auto-fired after PM finalizes.
- PO-only intake turn-taking (single agent per session).
- Long-poll endpoint (`GET /api/user-chats/{session_id}/poll`).
- Session routing: detect planning sessions by first message author, delegate to PM.
- Existing `/api/agents/{pm,db}/chat` paths removed (Phase 6).

### Phase 3: State Machine in Dispatcher ✅ DONE

- Dispatcher picks up tasks only where `status='ready'`.
- Specialist `complete_task` tool writes `status='done'`.
- EM `mark_task_ready` tool for `needs_technical_shaping → ready` (EM prompt may be stubbed initially — admin manual transition acceptable until EM is fully implemented).

### Phase 4: UI Migration ✅ DONE

- Replace chat drawer with single User Chat UI.
- Add guest user creation + display_name prompt.
- Add sessions list.
- Keep task chat view for execution visibility.
- Handoff detection: UI navigates to planning session when `handoff_session_id` is present.

### Phase 5: Comments ✅ DONE

- Comments API + UI.
- Role-based authorization.

### Phase 6: Decommission Legacy ✅ DONE

- Remove `/api/agents/{agent}/chat` user-facing endpoints.
- Remove per-agent chat pages in admin-gui.
- Keep internal `agent_chat_session` for execution traces.

### Phase 7+ (Future)

- EM prompt + EM joins user chat.
- PM edits existing epics/tasks.
- Real auth (email/password); guest claim flow.
- Voting / multi-PM models for diverse thinking.


## Risks and Tradeoffs

1. **Two-phase handoff friction** — PO hands off to PM, but PM may ask the user more questions. If the user has already left, the planning session stalls. Mitigation: PM should minimize follow-up questions and work from PO's summary.

2. **PO bottleneck** — single PO per intake session is a throughput cap. Mitigations (multiple PO instances, diverse models) are deferred.

3. **PO–PM disagreement** — PO validates *after* PM commits artifacts. If PO thinks artifacts are wrong, it must comment on the task, not undo. Acceptable now; tighten later (e.g., PO can transition back to `draft`) if friction emerges.

4. **State machine drift** — if specialists or the dispatcher mutate `status` outside their tools, the invariant breaks. Centralize state writes through helper functions with grep-able names.

5. **Guest identity loss** — clearing localStorage loses guest history. Document this; not blocking for beta.

6. **Session linking** — `handoff_session_id` creates a dependency between intake and planning sessions. If the planning session fails to start, the intake session is left completed with no follow-up.


## Open Questions

### Resolved

- Session reopening: terminal for now.
- Tasks per session: PM may create multiple.
- Epic creation: required (every intake produces an epic).
- PO rollout: sole intake agent from day one; hands off to PM for planning.
- PM scope: runs only in planning sessions; creates artifacts via `finalize_session`.
- EM scope: technical shaping; gates `ready`; will join chat later.
- Identity model: internal `user` table, integer FKs, guest → email/password upgrade path.
- Routing PM vs PO: session type detected by first message author; PO handles intake, PM handles planning.
- Initial-state policy: code function `initial_state_for(agent_type)`.
- Task draft semantics: PM creates as `draft`; PO transitions every task out after finalization.
- Attachments / rich text: plain text initially.
- Notifications: deferred.
- Handoff: PO calls `hand_off_to_pm` to create planning session and fire PM.
- Long-polling: `GET /api/user-chats/{session_id}/poll` replaces short-polling.
- Legacy endpoints: decommissioned (Phase 6 complete).

### Still Open

- **PO disagreement with PM's draft** — comment-only now, or can PO transition back to `draft` and request a redo? (Lean: comment-only.)
- **EM blocking** — if EM refuses to mark `ready`, what state? Back to `draft` with a comment, or a new `blocked` state?
- **Display-name collisions** — two guests pick the same name. Allow? Disambiguate in UI by `user.id`?
- **PM follow-up burden** — should PM be able to ask PO clarifying questions before finalizing, or work solely from the handoff summary?


## Proposed Next Steps

Phases 1–6 are complete. Remaining work:

1. **EM implementation** — prompt, `mark_task_ready` tool, technical shaping workflow.
2. **PM edits existing epics/tasks** — follow-up sessions that modify rather than create artifacts.
3. **Real auth** — email/password login; guest claim flow.
4. **PO can transition back to `draft`** — if PO disagrees with PM's artifacts, allow a redo request.
5. **Multi-PM / diverse thinking** — voting models for artifact quality.
