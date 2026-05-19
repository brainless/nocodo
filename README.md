# nocodo

**Talk to build software.** Describe what you want — an AI team turns it into a structured plan, epics, tasks, and working code.

> Website: [nocodo.com](https://nocodo.com) &nbsp;·&nbsp; Docs & support: [nocodo.com](https://nocodo.com)

---

## What is nocodo?

nocodo is a local-first desktop app with a built-in AI team. You have a conversation about what you want to build. Your AI Product Owner asks clarifying questions, hands off to your AI Project Manager, who creates epics and tasks. Specialist agents — backend, frontend, database, UI — pick up the work.

You describe the goal. nocodo figures out the plan.

---

## How it works

**Step 1 — Talk to the Product Owner**

Start a chat. The PO asks you about your users, what data you're tracking, which features matter, and what constraints you're working within. Some questions are free-text; others are click-to-answer widgets for speed. The PO does not create tasks — it listens and understands.

**Step 2 — PM creates the plan**

When the PO has enough clarity, it hands off to the Project Manager. The PM takes the requirements summary and creates a structured epic with tasks, each assigned to the right specialist agent.

**Step 3 — Specialists execute**

Tasks flow to backend, frontend, database, and UI designer agents. An Engineering Manager reviews and shapes technical tasks before they reach the specialists. Each task has a clear state: `draft → ready → in_progress → done`.

**Step 4 — Comment and refine**

Every epic and task has a comments thread. Specialists, PM, and PO use comments to surface questions and decisions. You stay in the loop without being in the way.

---

## Features

### Conversational requirements intake
Chat with an AI Product Owner who asks the right questions. Structured choice widgets (radio, checkboxes) keep answers precise and the context clean for downstream agents.

### Automatic epics and tasks
The Project Manager creates epics and tasks from the PO's requirements summary in a single atomic step — no manual decomposition, no copy-pasting notes into a tracker.

### Task state machine
Tasks move through a defined lifecycle: `draft → needs_technical_shaping → ready → in_progress → done`. The Engineering Manager gates technical tasks before they reach specialists. The rules are code, not config.

### Living project knowledge
nocodo maintains two persistent records as your project grows: **project notes** (business context, decisions, open questions) written by the PO, and **stack notes** (technical decisions, architecture) maintained by the EM. Future agents read these to stay in context.

### Comments on artifacts
Every epic and task is a conversation surface. Specialists comment on their assigned work. Users and PM/PO can read and reply. Technical discussion lives next to the work it's about.

### Local-first, open source
Everything runs on your machine. Conversations, artifacts, and agent state are stored in a local SQLite database. No cloud dependency, no lock-in.

---

## AI team roles

| Agent | What they do | Do you talk to them? |
|---|---|---|
| **Product Owner** | Requirements intake — listens, asks questions, summarises | Yes — this is your main conversation |
| **Project Manager** | Planning — creates epics and tasks from PO's summary | Yes — in the planning phase |
| **Engineering Manager** | Technical shaping — reviews tasks before specialists start | Not yet (coming soon) |
| **Backend Engineer** | Implements backend tasks | Via task comments |
| **Frontend Engineer** | Implements frontend tasks | Via task comments |
| **DB Engineer** | Designs and migrates database schemas | Via task comments |
| **UI Designer** | Produces UI specifications and component designs | Via task comments |

---

## Desktop app

nocodo is a [Tauri](https://tauri.app) desktop application. The AI backend, agent runtime, and admin UI all run locally.

- Mac, Windows, Linux
- SQLite database on disk — inspect or migrate freely
- No account required to start

---

## Roadmap

- **Engineering Manager** — fully wired: technical shaping, codebase reading, `needs_technical_shaping → ready` transitions
- **PM edits existing artifacts** — follow-up sessions that refine rather than recreate epics and tasks
- **Real auth** — email/password login; guest sessions that can be claimed
- **Generated web app** — TypeScript + SolidJS + TailwindCSS frontend generated from your project definition

---

## Tech stack

- **Backend** — Rust + Actix Web
- **Desktop** — Tauri wrapping the admin UI
- **Admin UI** — TypeScript + SolidJS + TailwindCSS + DaisyUI
- **Shared types** — Rust types with TypeScript codegen (no handwritten API contracts)
- **Agent runtime** — LLM-backed multi-agent system with SQLite-persisted sessions
- **Database** — SQLite (local default)

---

## Getting started

See [DEVELOP.md](DEVELOP.md) for local development setup, running the backend and UI, and the deploy workflow.

---

## Status

nocodo is in active development. The two-phase PO → PM chat flow, task state machine, specialist dispatch, and comments are all functional. Engineering Manager, real auth, and the generated web app are on the roadmap.

Contributions, feedback, and issues welcome at [github.com/brainless/nocodo](https://github.com/brainless/nocodo).
