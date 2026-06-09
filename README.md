# nocodo

> **Experimental · Work in Progress**
> A coding agent for small (<10B) local models, targeting a specific opinionated stack. Goals not yet fully achieved — but the early results are promising enough to keep going.

Coding agent built around small (<10B) — and in some cases *tiny* (<1B) — models for local development. The target stack is opinionated: **Rust (Actix Web + Diesel) on the backend, TypeScript (SolidJS + Tailwind) on the frontend**. Multiple agent roles — Product Owner, Project Manager, Engineering Manager, Rust Engineer, SolidJS Engineer, and more — will eventually collaborate to build full-stack apps. No coding knowledge required.

This will not produce a general-purpose coding agent. The bet is narrower and more achievable: **nocodo can get small-business CRUD apps built and managed entirely from a desktop app, using local LLMs, for anyone who can describe what they want.**

---

## The core idea

General coding agents need large, expensive models because they face open-ended problems. nocodo takes the opposite approach: **make each problem maximally narrow, then let a tiny model solve it reliably.**

The stack is fixed. The patterns are known. A Diesel model function, an Actix route handler, a SolidJS form component — each has a small, learnable shape. Instead of asking a model to figure things out, nocodo builds a dedicated agent mode for each discrete coding task, loads it with concrete examples, and asks for exactly one thing. The model copies a pattern. Deterministic post-processing cleans up the output.

The result: `Qwen3.5-0.8B` — a model that fits in under a gigabyte — generating correct Diesel ORM functions. That's the premise in action. It works because of the constraint, not despite it.

---

## How the stack gets covered

nocodo builds agent/mode coverage for each layer of the stack, one layer at a time. Each mode is a self-contained prompt builder: it extracts the right code context, constructs an example-rich single-shot prompt, and deterministically post-processes the output.

### Rust backend — current work

| Layer | Modes | Status |
|---|---|---|
| **Diesel ORM** | Model impl functions, model structs, schema definitions | ✅ Working |
| **Actix Web** | Controllers (handlers), routers, middleware | Planned |
| **Auth & permissions** | Session, JWT, role-based access | Planned |

### TypeScript frontend — next

| Layer | Modes | Status |
|---|---|---|
| **SolidJS** | Routing, state/context, forms, views | Planned |
| **Tailwind + DaisyUI** | Component styling | Planned |

### Platform layer — on the roadmap

| Capability | Description | Status |
|---|---|---|
| **Tooling management** | Install and configure Rust compiler, Node, and project dependencies — deterministic, no LLM involved | Planned |
| **Git integration** | GitHub/GitLab support for branches, commits, PRs — deterministic code, agent-managed | Planned |
| **Deployment** | VPS/SSH builds, or managed hosts (Render, Railway, Fly, Cloudflare) — fully API-integrated, same agent/mode pattern | Planned |

Once a few modes in a layer work reliably, adding new ones accelerates — because nocodo's own coding agents participate in building them.

---

## Why tiny models can work here

Most coding agents route around small models by using GPT-4 or Claude. nocodo deliberately constrains the problem so smaller models become viable:

- **Single-shot** — one prompt, one response, parse the output. No tool loops, no multi-turn reasoning.
- **Example-driven** — prompts contain 10–15 concrete patterns the model can copy, not instructions to reason from first principles.
- **Narrow scope per mode** — each mode generates exactly one artifact (one function, one struct, one route).
- **Deterministic post-processing** — imports are stripped and re-injected correctly, code fences are unwrapped, `<think>` blocks are discarded. The model doesn't need to get formatting right.
- **Low temperature** (0.1–0.2) — reproducibility over creativity.
- **Transparent output** — every run returns the prompt, raw model response, and extracted code for debugging.

The current `RustEngineer` agent runs `unsloth/Qwen3.5-0.8B-GGUF` via llama.cpp at `localhost:8080`. Override with `RUST_ENGINEER_MODEL` / `LLAMA_CPP_BASE_URL`.

---

## The AI team

nocodo also includes a coordination layer of higher-level agents designed to work with cloud or mid-size models. These handle the product and planning side — turning a plain-language description into structured epics and tasks that specialist coding agents execute.

| Agent | Role | Talks to users? |
|---|---|---|
| **Product Owner** | Requirements intake — listens, asks clarifying questions, summarises | Yes — main conversation |
| **Project Manager** | Planning — creates epics and tasks from PO's summary | Yes — planning phase |
| **Engineering Manager** | Technical shaping — reviews tasks before specialists start | Coming soon |
| **Backend Engineer** | Executes backend tasks | Via task comments |
| **Frontend Engineer** | Executes frontend tasks | Via task comments |
| **DB Engineer** | Database schema design and migration | Via task comments |
| **UI Designer** | UI specs and component designs | Via task comments |
| **Rust Engineer** | Diesel/Actix code generation — runs locally on tiny models | Admin tool |

The two-phase PO → PM chat flow, task state machine, specialist dispatch, comments, and code extraction are all functional. The coordination layer is ahead of the coding agents in maturity; the current focus is closing that gap.

---

## How it works (the full vision)

**Step 1 — Talk to the Product Owner.** Describe what you want to build. The PO asks about users, data, features, and constraints. Structured choice widgets (radio, checkboxes) keep the context clean.

**Step 2 — PM creates the plan.** The PO hands off to the Project Manager, who produces a structured epic with tasks assigned to the right specialists.

**Step 3 — Specialists execute.** Tasks flow to coding agents. The Engineering Manager reviews technical tasks before they reach specialists. State machine: `draft → needs_technical_shaping → ready → in_progress → done`.

**Step 4 — Platform handles the rest.** Tooling is installed, code is committed, the app is built and deployed — all managed by nocodo, all on infrastructure you control.

---

## Long-term goal

nocodo will get small-business CRUD apps **built, deployed, and managed** from a desktop app, using entirely local models, by people who can describe what they want in plain language. Auth and permissions, build and deployment pipelines, VPS or managed cloud — a full-stack development team as a product.

That's an ambitious claim for a project running on sub-gigabyte models. We're building toward it one agent mode at a time.

---

## Tech stack

- **Backend** — Rust + Actix Web
- **Desktop** — Tauri wrapping the admin UI
- **Admin UI** — TypeScript + SolidJS + TailwindCSS + DaisyUI
- **Shared types** — Rust types with TypeScript codegen (no handwritten API contracts)
- **Agent runtime** — multi-agent system with SQLite-persisted sessions
- **Database** — SQLite (local)
- **Code agents** — llama.cpp, models ≤ 1B for local private code generation

---

## Getting started

See [DEVELOP.md](DEVELOP.md) for local development setup, running the backend and UI, and the deploy workflow.

---

## Status

Active development. The `RustEngineer` — Diesel model functions, structs, and schema — is the current working example of tiny-model code generation. The PO → PM coordination flow is functional. Engineering Manager, real auth, and additional code agent modes are on the roadmap.

Contributions, feedback, and issues welcome at [github.com/brainless/nocodo](https://github.com/brainless/nocodo).
