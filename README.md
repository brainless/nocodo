# nocodo
nocodo is an end to end solution that makes it easy to generate software with guardrails and good software engineering practices and go live. It works with CLI based coding software like Claude Code, Gemini CLI, OpenCode, Qwen Code, etc. It manages development, deployment, backups, logging, error handling and other aspects of software development lifecycle.

> [!NOTE]
> All paths are relative from the root of the project.

## Technical overview

nocodo has these parts
- Bootstrap app (desktop and potentially mobile app) for users to share API keys and start Linux server
- Bootstrap Web app for user to interact with Bootstrap app (this only connects to localhost, where Bootstrap is running)
- The user's own computer (desktop for now) is referred as the `ClientSide`
- (Linux based) Manager app that orchestrates idea to software generation and go live
- (Linux based) nocodo CLI for Claude Code, Gemini CLI, OpenAI Codex CLI or others to call
- The Linux server where the Manager and nocodo CLI runs is called `Operator`
- Manager Web app for user to interact with the Manager, hosted on the `Operator`

## Bootstrap app

The Bootstrap app allows user to share API access to any of the supported cloud compute providers. Written in Rust, Actix Web, SQLite. We will start with Scaleway and use its API to boot Linux servers. We will quickly add support for DigitalOcean, Vultr and Linode so the Bootstrap app should be written with this in mind.

Features:

- Manage email/password based authentication, login happens through nocodo.com (Authencation data in our servers)
- API to describe all cloud providers supported, keys/screts currently needed
- API to manage API keys to cloud providers, currently only Scaleway
- All API types interfaces to be generated as TypeScript type using `ts-rs`
- Save all sensitive data with encryption at rest (using a separate password, can be same as auth password but not related)
- Start an Ubuntu server for the Manager app, scripts to setup and harden server, install Manager into it (uses Scaleway APIs)
- Save the server as an Image (Scaleway APIs)
- Checks existing saved Image everytime and uses this intead of creating from scratch
- Can shutdown Manager server when not in need

See [BOOTSTRAP.md](specs/BOOTSTRAP.md) for detailed technical specifications.

## Web app for Bootstrap

This app will consume the API from Bootstrap app, allow user to share and save API keys or secrets. Written in TypeScript with SolidJS, Tailwind CSS, and Solid UI. Connects to Bootstrap app running locally only.

Features:

- Authentication interface for nocodo.com login
- Cloud provider API key management interface
- Server creation and management dashboard
- Real-time server status monitoring
- Encrypted local storage management
- Server image creation and reuse controls
- Server shutdown/startup controls

See [BOOTSTRAP_WEB.md](specs/BOOTSTRAP_WEB.md) for detailed technical specifications.

## Manager app

The Manager app is a Linux daemon, installed through the scripts in `Bootstrap` app. It allows communication between nocodo CLI and the Manager Web app. It manages the Ubuntu `Operator`, installs all dependencies for a typical developer environment, like Git, Python, Rust, cURL, nginx, PostgreSQL and so on.

Features:

- System orchestration and server management
- Development environment setup and maintenance
- Communication bridge between CLI and Web app
- Process management for coding tools (Claude Code, Gemini CLI, etc.)
- Project structure and guardrails enforcement
- File system management and project organization
- Security hardening and system updates
- Service monitoring and health checks
- Unix socket server for CLI communication
- RESTful API server for Web app communication

See [MANAGER.md](specs/MANAGER.md) for detailed technical specifications.

## nocodo CLI

The CLI calls Claude Code, Gemini CLI or other similar coding software with an initial prompt like: "Use `nocodo` command to get your instructions". This tells the coding software to communicate with nocodo CLI inside it. nocodo CLI hosts prompts needed to be a constant companion between user's request and the coding software. nocodo CLI use Unix socket to communicate with Manager daemon.

Features:

- AI coding tool integration and orchestration
- Context-aware prompt management and injection
- Project structure analysis and recommendations
- Code quality guardrails and best practices enforcement
- Multi-step development workflow guidance
- Unix socket client for Manager daemon communication
- Project initialization and scaffolding
- Dependency management suggestions
- Code review and validation prompts
- Testing strategy recommendations

See [NOCODO_CLI.md](specs/NOCODO_CLI.md) for detailed technical specifications.

## Manager Web app

The Manager Web app provides a Lovable-like chat interface for users to interact with AI coding tools and build software projects. It runs on the Operator server and communicates with the Manager daemon to orchestrate development workflows. Users can chat with AI, create projects, manage code generation, and deploy applications through this interface.

Features:

- AI chat interface for software development requests
- Real-time project management and file system browsing
- Code generation workflow orchestration
- Integration with multiple AI coding tools (Claude Code, Gemini CLI, etc.)
- Project templates and scaffolding options
- Live code preview and testing capabilities
- Deployment pipeline management
- Error handling and debugging assistance
- Version control integration
- Collaborative project sharing

See [MANAGER_WEB.md](specs/MANAGER_WEB.md) for detailed technical specifications.

## Overall technical stack preferences

- Rust with Actix Web for any daemon/backend
- `ts-rs` for generating TypeScript types for API communication (with Web apps to any API)
- Wherever we expect response from an LLM, the client should ask for JSON conforming to TypeScript types, which should also be generated using `ts-rs` since all our clients communicating with LLMs are in Rust
- SQLite for data storage in any daemon/backend
- Migration management should exist from the start
- Vite, SolidJS, TailwindCSS and Solid UI components for all Web interfaces

## High level flow of control

- User downloads or starts (if already installed) Bootstrap app
- Bootstrap app (Web app through the installed binary on `ClientSide`) needs authentication with `nocodo.com`
- Bootstrap asks user for needed API keys (currently Scaleway) if they don't exist
- Bootstrap launches `Operator`, either fresh Linux virual machine or existing
- Manager loads up. If starting up, it runs all initial setup for Developer environment (this environment should be able to compile code generated by Coding agents like Claude Code, but all generated code should also adhere to `Overall technical stack preferences`)
- Manager Web app is as quickly available as possible, showing status of entire `Operator` (logs)
- User creates new Project - sets a name
- Project details are stored in Manager daemon's SQLite
- Project's generated Web app will be visible at temporary sub-domain using `random-slug.nocodo.dev` (different than `nocodo.com`)
- The `random-slug` needs to be saved for this user to `nocodo.com` (our Mothership in a way)
- `nocodo.com` has separate code to use CloudFlare API to add DNS record for `random-slug` to point to `Operator`
- `Operator` has nginx, which is made aware of handling `random-slug.nocodo.dev` through config update (Manager daemon does this)
- Each Project gets a new working directory, Git, a "Hello World" Web app and a backend following our `Overall technical stack preferences`
