# nocodo

An opinionated, agent-enabled, full-stack application engine. Say what you want to build (business automation, personal finance, project management and so on) and get ready-to-use software. Built with Rust, SQLite and SolidJS.

> **Very early stage.** nocodo is an experiment, not a finished product. Today it does **one thing**: it turns a plain-language description into a real database design (the storage layer) and builds that database for you. Import, rules, reports and a finished app UI come later. Expect rough edges and changes between versions.

## What you can do today

Describe what you want to keep track of, in your own words:

> I want to keep an eye on my family's money. My salary comes in every month, we spend on the household, and sometimes I lend money to my brother. We have two bank accounts.

nocodo then:

1. **Asks a few questions** (at most 8, in one round), each with numbered choices and a suggested default. No technical jargon: you are never asked about tables, keys or SQL.
2. **Designs the storage** for your data as a proper relational SQLite database, not a pile of loose notes.
3. **Checks its own work** before touching anything: it test-runs the change on a copy of your data first.
4. **Backs up your data, then applies the change** to `data.sqlite`.
5. **Tells you in plain language** what your tool will now remember.

Later you can ask for more ("I'd also like to know who in the family each spend was for"). nocodo adds to the existing design and keeps the data you already have. If a change would remove data, it asks you to confirm first, and nothing happens until you say yes.

Everything stays on your machine, in one folder per project.

## Install

Prebuilt downloads will be on the [GitHub Releases](../../releases) page. They are **coming soon**; we are setting up automated builds now. You will not need Rust or any developer tools.

Each release will contain three programs:

| Program | What it is |
|---|---|
| `nocodo-gui` | A desktop window: the easiest way to start. |
| `nocodo` | The command line, for scripts and terminal fans. |
| `nocodo-api` | A local server the window talks to. `nocodo-gui` starts one for you. |

## Before you start: a model key

nocodo uses an AI model to understand your description. You bring your own key. Create a file named `.env` in the folder you run nocodo from and add the key for the provider you use, for example:

```
ANTHROPIC_API_KEY=your-key-here
```

Supported variables: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, `OPENROUTER_API_KEY`, `DEEPSEEK_API_KEY`, `NEURALWATT_API_KEY`. Local models through llama.cpp need no key. Which provider and model each project uses is set in its `project.json`.

The AI only proposes a design. It never writes SQL or touches your data directly; ordinary, predictable code does that.

## Using the window

1. Start `nocodo-gui`.
2. Pick **New project...** and give it a name.
3. Type what you want to keep track of in the prompt box.
4. Answer the questions that appear, or click **Use defaults**.
5. Look at the summary and your tables in the main area. **Add sample rows** fills them with fake rows so you can see how it will look.

## Using the command line

```bash
# Make a project
nocodo init --project my-money

# Describe what you want (or use --file prompt.md)
nocodo prompt --project my-money "I want to keep an eye on my family's money..."

# If it asks questions
nocodo questions --project my-money
nocodo answer --project my-money q1=2 q3=1     # choices are numbered from 1
nocodo answer --project my-money --defaults    # accept suggested answers

# See what you have
nocodo inspect --project my-money

# Try it with fake rows
nocodo sample-rows --project my-money
```

If a model call fails, `nocodo continue --project my-money` retries it. Set `NOCODO_PROJECT` to skip repeating `--project`.

## What is inside a project

A project is just a folder:

- `data.sqlite`: your data, a standard SQLite file you can open with any SQLite tool.
- `specs/schema.json`: the design of your data.
- `migrations/`: every change ever made to the design, as plain SQL.
- `backups/`: a copy of your data from before each change.
- `events.jsonl`: a readable history of everything that happened and why.

## Current limits

- Storage only: no importing files, rules or reports yet.
- One person, on one machine. No accounts, no cloud, no editing rows in the window yet.
- SQLite only.
- A project accepts one change at a time.

## How it works (short version)

nocodo uses small, focused agents that each own one job and produce **typed specs, not code**. Deterministic Rust code turns specs into real artifacts (like SQL migrations), and a prebuilt runtime runs them. That is why the same request gives predictable results and why you never need a developer toolchain.

- [`PRD.md`](PRD.md): what the current experiment tests and why.
- [`AGENTS.md`](AGENTS.md): rules and crate boundaries for contributors and coding agents.
- [`epics/`](epics/): the step-by-step build plan.

## License

GNU Affero General Public License v3.0. See [`LICENSE`](LICENSE).
