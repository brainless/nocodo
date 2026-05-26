# Multi-Agent Design: Two-Phase User Chat (PO Intake → PM Planning), Task State Machine, and Comments

## Summary

This document captures the design for:

1. A two-phase user chat flow: Product Owner (PO) handles requirements intake alone, then hands off to Project Manager (PM) for planning.
2. Replacing the agent-centric chat drawer with a single user-facing chat session.
3. PO as the sole intake agent — PM is NOT present during requirements gathering.
4. PO gathers requirements then signals completion via `complete_requirements` (requirements gathering mode) and names the project via `set_project_name` (project naming mode). The backend auto-creates a planning session seeded with project notes and fires PM once.
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

| Role | Scope | User-facing? | Model class |
|------|-------|--------------|-------------|
| `product_owner` (PO) | Requirements intake. Listens to user, gathers requirements via text and structured questions. Calls `complete_requirements` when done, then `set_project_name` in a separate naming call. Validates tasks after PM finalizes. | Yes — sole agent in intake session | Cloud (Anthropic/Groq/OpenAI) |
| `project_manager` (PM) | Planning and artifact creation. Runs only in the planning session (created by PO handoff). Asks follow-up questions if needed. Calls `finalize_session` to atomically create epic + tasks. | Yes — in planning session only | Cloud (Anthropic/Groq/OpenAI) |
| `engineering_manager` (EM) | Technical shaping. Gates `needs_technical_shaping → ready` for tasks that need it. May read codebase. | No initially; will join user chat later. | Cloud (Anthropic/Groq/OpenAI) |
| `db_engineer`, `backend_engineer`, `frontend_engineer`, `ui_designer` | Execute `ready` tasks assigned to them. | No — interaction via task comments only. | Small (≤ 20B) |
| `rust_engineer` | Generates Rust/Diesel ORM impl functions by example. Single-shot prompt → code. Uses tree-sitter extraction for struct definitions, existing impl functions, and dependent types. | No — admin UI tool for code generation | Tiny (≤ 1B, llama.cpp local) |

### 1a) Small-Model Code Agents

Code writer agents (`db_engineer`, `backend_engineer`, `frontend_engineer`, `ui_designer`, `rust_engineer`) use small or tiny models (≤ 20B parameters) to keep inference cheap and local-friendly. This constraint shapes their design:

**Design principles for small models:**
- **Example-driven prompts** — show concrete patterns, never ask the model to "figure it out"
- **Deterministic post-processing** — strip and re-inject imports, unwrap code fences, strip `<think>` blocks
- **Single-shot where possible** — avoid multi-turn tool loops; one prompt → one response → parse
- **Narrow scope per mode** — each mode does one thing (e.g. `diesel_model_fn` writes one function body)
- **Low temperature** (0.1–0.3) — favor determinism over creativity
- **Transparent output** — return prompt + raw response + extracted code for debugging

**`rust_engineer`** is the most constrained: runs on `Qwen3.5-0.8B` via llama.cpp at `localhost:8080`, single-shot, no tools, max 512 tokens. Override with `RUST_ENGINEER_MODEL` / `LLAMA_CPP_BASE_URL`.

### 2) Two-Phase User Chat Surface

The existing chat drawer (PM/DB/BE/FE peer contacts) is removed.

There are two chat phases per user-facing initiative:

**Phase 1 — Intake session (PO only):** The user describes what they want to build. PO asks clarifying questions (text and structured widgets) to understand the business context, users, data, features, and constraints. PO does NOT create artifacts. When PO has enough clarity, it calls `complete_requirements`, then names the project via `set_project_name` in a separate call.

**Phase 2 — Planning session (PM only):** Created automatically by the handoff. Seeded with PO's requirements summary. PM may ask follow-up questions. When PM has enough clarity, it calls `finalize_session` to create the epic and tasks atomically.

PM and PO are NEVER in the same session concurrently. The backend detects planning sessions by checking if the first message is from `product_owner` and routes accordingly (`run_pm_planning` vs `run_po_intake`).

Persistent tables:

- `user_chat_session` (with `handoff_session_id` linking intake → planning)
- `user_chat_message`

Agent-internal chat is preserved for task execution (`agent_chat_session` / `agent_chat_message`) — used by the dispatcher and by specialists during task work, not exposed as user entry points.

Technical conversation about a specific artifact happens on **task/epic comments**, not in chat. This is the principle: chat is for product conversation with non-technical roles; comments are for execution discussion anchored to artifacts.

### 3) User Chat Access Policy

- PO: read/write to intake sessions. Calls `complete_requirements` + `set_project_name` to signal completion; backend transitions to planning automatically.
- PM: read/write to planning sessions only. Calls `finalize_session` to create artifacts.
- EM: future.
- Specialists (DB/BE/FE/UI): no direct access. Reached via task comments.

### 4) Session Lifecycle

**Intake session:**
- A new user chat starts a new `user_chat_session` (intake).
- While the session is `open`, the user may append messages and PO continues to respond.
- PO gathers requirements via text and structured questions (`request_user_input`), recording facts via `record_project_note`.
- When PO has enough clarity it calls `complete_requirements { closing_message }` (requirements gathering mode), then `set_project_name { name }` (project naming mode — separate LLM call).

**Completion (backend-driven):**
1. PO's `closing_message` is stored in the intake session.
2. PO's `set_project_name` call renames the project.
3. A new planning session is created.
4. Current project notes are seeded as the first message in the planning session (from `product_owner`).
5. `handoff_session_id` is set on the intake session, linking it to the planning session.
6. The intake session is marked `completed`.
7. PM is fired once in the background in the planning session.

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
4. PO has four possible outcomes per turn:
   - **Text response** (`PoSessionResult::Text`) — stored as a message with `agent_type = product_owner`.
   - **Structured questions** (`PoSessionResult::Questions`) — one or more `structured_question` messages stored; backend waits for user answers before firing PO again.
   - **Requirements complete** (`PoSessionResult::RequirementsComplete`) — closing message stored; backend immediately calls PO again in `project_naming` mode.
   - **Named** (`PoSessionResult::Named`) — `set_project_name` was called; backend triggers PM handoff.

**Unanswered structured question guard:** If there are `structured_question` messages without matching `structured_response` messages, the backend does NOT fire PO. PO runs only after the user has answered all pending questions.

### Phase 2: PM Planning (Single Agent)

Triggered automatically after PO's project naming mode returns `Named`:

1. Backend creates a new planning session and seeds it with PO's summary.
2. Backend fires **PM only** (`run_pm_planning`).
3. PM may respond with text, structured questions, or call `finalize_session`.
4. After PM finalizes (atomic: message + epic + tasks + session completion), PO validates all created tasks.

**Session routing:** The backend detects planning sessions by checking if the first message is from `product_owner`. If so, it delegates to `run_pm_planning` instead of `run_po_intake`. This ensures PO doesn't respond alongside PM in planning sessions.

### Completion Flow

PO completion is a two-step process, each a separate LLM call:

**Step 1 — Requirements gathering mode:** PO calls `complete_requirements { closing_message }`.

```rust
struct CompleteRequirementsParams {
    closing_message: String,  // warm closing message shown to the user
}
```

Backend:
1. Stores `closing_message` in the intake session and notifies the UI.
2. Immediately calls PO again in `project_naming` mode (same session, same message history).

**Step 2 — Project naming mode:** PO calls `set_project_name { name }`.

```rust
struct SetProjectNameParams {
    name: String,  // concise domain-derived name, ≤ 60 chars
}
```

Backend (`handle_po_complete`):
1. Renames the project.
2. Creates a new `user_chat_session` (planning session).
3. Seeds planning session with current project notes as the first message (author: `product_owner`).
4. Sets `handoff_session_id` on the intake session → links to planning session.
5. Marks intake session `completed`.
6. Fires PM in the planning session in the background.

PO never references PM — it signals completion and names the project; the backend handles everything else.

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
6. PO reads the compiled answer and continues gathering requirements or calls `complete_requirements`.

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

`handoff_session_id` is set by the backend after PO's project naming completes, linking the intake session to the newly created planning session.

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


## Agent Mode Architecture

### What Is a "Mode"?

An agent **mode** is a named activation context that changes three things simultaneously
for the same agent:

1. **System prompt** — what the agent is told about its current situation and goals.
2. **Available tools** — which tools the agent may call in this context.
3. **Data access** — what project context (tasks, schema, PO summary, etc.) is injected.

"Mode" is preferred over "persona" (which implies a different identity) or "situation"
(informal). The same physical agent (same code, same struct) runs in different modes
depending on how and when it is invoked.

### Module Structure

Each agent that has more than one mode uses a `modes/` sub-directory:

```
agents/src/{agent_name}/
├── mod.rs
├── agent.rs        ← selects a mode and drives the LLM loop
├── tools.rs        ← tool parameter types (shared across modes)
└── modes/
    ├── mod.rs      ← re-exports all mode modules
    ├── core.rs     ← shared identity + principles (the "heart" of the agent)
    └── {mode}.rs   ← one file per mode; each composes core.rs
```

### `core.rs` — Single Source of Truth

`core.rs` contains a function (e.g. `pm_core()`) that returns the invariant part of the
agent's system prompt: its identity, non-negotiable rules, and shared guidelines. Every
mode's `system_prompt()` calls this function and formats the mode-specific section around
it. Editing `core.rs` affects all modes simultaneously.

Example structure for a mode file:

```rust
// modes/init.rs
use super::core::pm_core;

pub fn system_prompt() -> String {
    format!(
        r#"{core}

## Mode: Project Initialisation
...mode-specific instructions...
"#,
        core = pm_core()
    )
}
```

### PO Modes (Current)

| Mode | File | When used | Tools available |
|------|------|-----------|-----------------|
| `requirements_gathering` | `modes/requirements_gathering.rs` | Every user turn during intake | `request_user_input`, `record_project_note`, `complete_requirements` |
| `project_naming` | `modes/project_naming.rs` | Single call by backend after `RequirementsComplete` | `set_project_name` |

The two modes are separate LLM calls. `requirements_gathering` mode drives the full intake
conversation; `project_naming` mode fires once at the end to name the project from the
conversation history. PO has no knowledge of PM — the backend handles the transition.

### PM Modes (Current)

| Mode | File | When used | Tools available |
|------|------|-----------|-----------------|
| `init` | `modes/init.rs` | First message of a brand-new project (dead code — superseded by PO flow) | `create_epic`, `create_task` |
| `general` | `modes/general.rs` | Ongoing PM session — triage, status | `list_pending_review_tasks`, `create_epic`, `create_task`, `update_task_status` |
| `user_session` | `modes/user_session.rs` | Direct user requirements chat | `finalize_session`, `request_user_input` |
| `po_handoff` | `modes/po_handoff.rs` | Planning session after PO handoff | `finalize_session`, `request_user_input` |

`agent.rs` selects the mode and constructs the `CompletionRequest` with the matching
`system_prompt()` and tool list. No mode logic lives in `agent.rs` itself — it is a
dispatcher to mode functions.

### Adding a New Mode

1. Create `modes/{mode_name}.rs` with a `system_prompt() -> String` function that calls
   `super::core::{agent}_core()` and appends mode-specific instructions.
2. Declare it in `modes/mod.rs` (`pub mod {mode_name};`).
3. Add a call site in `agent.rs` that selects the new `{mode_name}::system_prompt()` and
   the correct tool list.

### Extending to Other Agents

Any agent with more than one activation context should adopt the same layout. The `core.rs`
function becomes the canonical description of that agent's identity — the equivalent of a
job description that never changes regardless of which task the agent is doing.


### RustEngineer — Constrained Code Generation

`RustEngineerAgent` is a single-shot code generator optimized for tiny local models
(≤ 1B parameters). It does NOT use tools, multi-turn conversation, or cloud LLMs.
Instead it extracts code context via tree-sitter, builds a self-contained prompt with
concrete examples, and parses the response deterministically.

**Model:** `unsloth/Qwen3.5-0.8B-GGUF:UD-Q4_K_XL` via llama.cpp at `localhost:8080`.
Override with `RUST_ENGINEER_MODEL` / `LLAMA_CPP_BASE_URL`.

**Design principles:**
- **Single-shot** — one prompt → one response → parse. No tool loops.
- **Example-driven** — prompt contains 13+ concrete patterns the model can copy.
- **Deterministic post-processing** — strips model `use` lines, prepends correct imports
  from `#[diesel(table_name)]`, unwraps code fences, strips `<think>` blocks.
- **Narrow scope per mode** — each mode generates one function body.
- **Low temperature** (0.2) — favor determinism over creativity.
- **Transparent** — returns `prompt + raw_response + extracted_code` for debugging.

**Module:** `agents/src/rust_engineer/`

```
agents/src/rust_engineer/
├── mod.rs              ← re-exports
├── agent.rs            ← RustEngineerAgent struct, diesel_model_fn() entry point
└── modes/
    ├── mod.rs
    └── diesel_model.rs ← build_prompt() + extract_table_name()
```

**Backend endpoint:** `POST /api/rust-engineer/run` (synchronous, runs inline).

**Admin UI:** `RustEngineerPage` — struct/fn inputs in top nav, three result panels
(prompt sent, raw response, extracted code). Wrench icon in sidebar.

#### Current Modes

| Mode | File | Input | Output |
|------|------|-------|--------|
| `diesel_model` | `modes/diesel_model.rs` | `struct_name`, `fn_name` | Diesel impl function body for SQLite |

**`diesel_model` mode flow:**
1. `find_struct_file()` → locate the `.rs` file containing the struct
2. `extract_struct()` → get the struct definition via tree-sitter
3. `list_impl_fns()` → collect up to 3 existing impl functions as style examples
4. `find_dependent_types()` → discover custom enums referenced in struct fields
5. `extract_table_name()` → parse `#[diesel(table_name = X)]` for deterministic imports
6. `build_prompt()` → compose single-shot prompt with rules, examples, struct, dependent types
7. LLM call → `max_tokens: 512, temperature: 0.2`
8. `extract_code()` → strip `<think>`, unwrap fences
9. `prepend_imports()` → strip model's `use` lines, prepend `use diesel::prelude::*; use crate::schema::{table};`

**Prompt composition** (`build_prompt`):
- 9 explicit Diesel+SQLite rules (connection type, insert/update/delete patterns, select/returning, SQLite types)
- 4 struct/derive examples (Queryable, Insertable, Associations, AsChangeset)
- 13 CRUD examples (insert via struct/columns/tuple, query with filter/find/limit/order, update single/multiple/changeset, delete by ID/pattern, relationships via belonging_to/inner_join)
- Target struct definition
- Dependent type definitions (enums, etc.)
- Up to 3 existing impl functions for style matching
- Final instruction: "Write ONLY the function definition. No imports, no explanation, no markdown fences."

#### Planned Modes (Experimental)

If `diesel_model` proves reliable with tiny models, additional modes will be added for
the full Actix Web + Diesel stack:

| Mode (planned) | Input | Output |
|----------------|-------|--------|
| `actix_controller` | `struct_name`, handler name, HTTP method | Actix `#[get/post/put/delete]` handler function |
| `actix_router` | module path, handler names | `ServiceConfig` route registration |
| `shared_type` | struct name, fields | `shared-types` Rust struct with `#[derive(TS)]` |
| `solid_component` | component name, props | SolidJS component with TypeScript types |

Each mode follows the same pattern: extract context → build example-rich prompt → single-shot → deterministic post-processing.

**Adding a new mode:**
1. Create `modes/{mode_name}.rs` with `build_prompt(...) -> (String, ...)` function
2. Declare in `modes/mod.rs`
3. Add method on `RustEngineerAgent` in `agent.rs` that extracts context, calls `build_prompt`, fires LLM, post-processes
4. Add backend handler in `backend/src/agents_api/rust_engineer/`
5. Add admin UI controls on `RustEngineerPage`


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

- **PO**: `request_user_input(question, input_type, options)` — poses structured choice questions during intake. `record_project_note(topic, title, note, replaces_note?)` — records requirements facts as project notes. `complete_requirements(closing_message)` — signals end of requirements gathering; backend calls PO in project naming mode next. `set_project_name(name)` — names the project (project naming mode only); backend then creates the planning session and fires PM. `validate_tasks(task_ids)` — transitions all PM-created tasks out of `draft` using `initial_state_for`.
- **PM**: `finalize_session(final_message, epic_title, epic_description, tasks)` — atomically commits artifacts + completes the planning session in one transaction. `request_user_input(question, input_type, options)` — poses structured choice questions during planning.
- **EM**: `mark_task_ready`, `comment_on_task`.
- **Specialist**: `complete_task`, `comment_on_assigned_task`.

The `request_user_input` tool is defined in `agents/src/user_input_tool.rs` and shared. It is not agent-specific.

### Prompts

Prompts are organised using the Agent Mode Architecture described above.

- **PO**: organised under `agents/src/product_owner/modes/`. `core.rs` holds the invariant PO identity (warm, empathetic, non-technical, MVP-first). `requirements_gathering.rs` drives intake — gather business context, users, data, features, constraints; use `request_user_input` for structured choices; record facts immediately with `record_project_note`; call `complete_requirements` when done. `project_naming.rs` is a focused, single-call mode: derive a project name from the conversation history and call `set_project_name`. Never refer to internal roles or PM to the user.
- **PM**: organised under `agents/src/project_manager/modes/`. `core.rs` holds the invariant PM identity; each mode file adds its activation-context-specific instructions. See the Agent Mode Architecture section for the full mode table.
- **EM**: technical shaping (TBD content); read codebase as needed; transition `needs_technical_shaping → ready`.
- **RustEngineer**: organised under `agents/src/rust_engineer/modes/`. No `core.rs` — modes are self-contained prompt builders. Each mode extracts code context via tree-sitter, builds an example-rich single-shot prompt, and returns `(prompt, metadata)`. The agent fires the LLM and post-processes deterministically. See the RustEngineer section above for mode details.

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
- Implement PO `complete_requirements` + `set_project_name` two-mode flow — backend creates planning session seeded with project notes, fires PM. (`hand_off_to_pm` removed; PO no longer references PM directly.)
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
- PO completion: two-mode flow — `complete_requirements` (requirements gathering) then `set_project_name` (project naming); backend creates planning session and fires PM automatically.
- Long-polling: `GET /api/user-chats/{session_id}/poll` replaces short-polling.
- Legacy endpoints: decommissioned (Phase 6 complete).

### Still Open

- **PO disagreement with PM's draft** — comment-only now, or can PO transition back to `draft` and request a redo? (Lean: comment-only.)
- **EM blocking** — if EM refuses to mark `ready`, what state? Back to `draft` with a comment, or a new `blocked` state?
- **Display-name collisions** — two guests pick the same name. Allow? Disambiguate in UI by `user.id`?
- **PM follow-up burden** — should PM be able to ask PO clarifying questions before finalizing, or work solely from the handoff summary?


## Code Extractor

Tree-sitter-based code extraction for coding agents (small models, precise context).

**Module:** `agents/src/code_extractor/`

**Two modes:**
- **Single-file extraction** — `extract_struct(path, "ContactRecord")`, `extract_free_fn(path, "register_user")`, `extract_impl_fn(path, "ContactRecord", "find_by_email")`, `extract_enum(path, "ContactType")` — returns `CodeBlock { file, start_line, end_line, source }`
- **Indexed lookup** — `CodeIndex::open("code_index.db")?; idx.build(&root)?;` — SQLite-backed, instant queries via `get_struct()`, `get_free_fn()`, `get_impl_fn()`, `list_impl_fns()`
- **Dependency scanning** — `find_dependent_types(project_path, source_file, struct_source)` — parses struct fields to discover custom enum/type references, then extracts their definitions. Used by RustEngineer to include dependent types in prompts.

**SQLite schema** (stored in project DB, commit-friendly):
- `code_index_structs` — name PK, file, start_line, end_line, source
- `code_index_free_fns` — name PK, file, start_line, end_line, source
- `code_index_impl_fns` — (struct_name, fn_name) PK, file, start_line, end_line, source

**Design:** Inherent impls only (skips `impl Trait for Struct`). Agents query the index to check if helpers exist, then retrieve exact function bodies — no file I/O at query time. `reindex_file()` for incremental updates.


## Proposed Next Steps

Phases 1–6 are complete. Remaining work:

1. **EM implementation** — prompt, `mark_task_ready` tool, technical shaping workflow.
2. **PM edits existing epics/tasks** — follow-up sessions that modify rather than create artifacts.
3. **Real auth** — email/password login; guest claim flow.
4. **PO can transition back to `draft`** — if PO disagrees with PM's artifacts, allow a redo request.
5. **Multi-PM / diverse thinking** — voting models for artifact quality.
6. **RustEngineer mode expansion** — add `actix_controller`, `actix_router`, `shared_type`, `solid_component` modes if `diesel_model` proves reliable with tiny models (≤ 1B).
7. **Small-model specialist agents** — extend tiny-model pattern to `db_engineer`, `backend_engineer`, `frontend_engineer` task execution (≤ 20B).
