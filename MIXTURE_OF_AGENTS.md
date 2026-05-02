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
2. **Focused agents write tasks for themselves only** — `create_task`, `update_task_status` are formal output steps, not optional.
3. **Every significant output is a task update** — schema ready, API contract defined, component done.
4. **PM has a `list_pending_review_tasks` tool** — used at session start to triage work created via direct-prompt paths.
5. **Tasks carry `source_prompt`** — the original user intent verbatim, so focused agents have full context without reading PM's chat.

## Task Model (draft)

```
Epic
  id, project_id, title, description, source_prompt, status, created_by_agent

Task
  id, epic_id, project_id, title, description, source_prompt,
  assigned_to_agent, status, depends_on_task_id, created_by_agent
```

Statuses: `open → in_progress → review → done | blocked`

## Rollout Plan

1. **Epic/Task storage** — SQLite tables, `TaskStorage` trait, migrations
2. **DB Developer tools** — add `create_task` / `update_task_status` as tool calls; test end-to-end
3. **PM Agent** — new agent with `create_epic`, `create_task`, `assign_task`, `list_pending_review_tasks`; test PM → DB Developer hand-off
4. **Backend Developer Agent** — add agent; test DB → Backend cross-agent dependency
5. **Frontend Developer Agent** — add agent; test full-stack Epic flow
