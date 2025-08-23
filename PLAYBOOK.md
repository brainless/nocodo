# Playbook for vibe coding an MVP web application

### Using terminal-based coding tools (Claude Code, Gemini CLI, Continue.dev CLI, Qwen Code CLI, Cursor CLI, etc.) and a structured prompting flow where you are the Product Owner and Project Manager

## Starting a Project

* Create a project, add a `specs` folder and `PROJECT.md` in it.
* Start with your business problem.
* Voice record user(s) flow, transcribe (Google AI Studio), and create a Markdown file.
* Save to `specs/THOUGHTS.txt`.
* Prompt any coding agent: "Take transcription and write `specs/PROJECT.md`, focus on MVP only."
* Prompt: "Read `specs/PROJECT.md`, add `Questions for Clarification`, with questions and default answers focusing on business, UI/UX, user journey, and similar details for MVP. I will edit answers if needed."
* Check `specs/PROJECT.md`, see the section `Questions for Clarification`, and edit answers as needed.
* Prompt: "We want reliable software, typed languages for backend/frontend apps only. API payload types should be generated from the backend. Deployment automation using infrastructure as code and GitHub-based CI/CD. We want end-to-end and integration tests, code format, lint checks through CI/CD. Read `specs/PROJECT.md` and create additional documents in the `specs/` folder where we need to capture business details like `Choices for XYZ field in API/UI` or technical details like `Deployment Plan`, `Backup Plan`, `Testing Plan`. Link the created documents to `specs/PROJECT.md`."
* Prompt: "Read documents in `specs/`, create `CLAUDE.md` for Claude Code, link to documents in `specs/` as needed. Create `README.md` for Product Owners and general users."
* I will paste `Development Workflow` (see below) into `CLAUDE.md`; change it as you need.
* Prompt: "Let's create a GitHub issue to build the backend and web app with a basic 'Hello World' API between them so we can manually test the apps and integration. Follow `specs/PROJECT.md` and linked documents for technical inputs. Mention in the issue: Types in API must be generated from the backend. Use the `gh` command for GitHub."
* Work on the first issue using the prompts in `Initite Development`

## Initiate Development

* Prompt referring to the GitHub issue: "Please attempt GitHub issue <__>, use the `gh` command, follow `Development Workflow` in `CLAUDE.md`. Follow `specs/PROJECT.md` and linked documents for technical inputs. When you want me to test, let me know; I will run the backend and web apps myself."
* You should test manually, running backend and web applications locally.
* If you have errors, see `Tackle Errors`, get errors fixed, making sure you can test this "Hello World" version of your app.
* Prompt: "Let's create a GitHub issue to create a GitHub Actions workflow for code quality checks, format, and lint (backend and web app). Also, a workflow for GitHub Copilot review. Follow `specs/PROJECT.md` and linked documents for technical inputs. Use the `gh` command for GitHub."
* Optionally, but much more technical and depends on what infrastructure you want to use: "Create a GitHub issue to create deployment code/scripts for this project. Follow `specs/PROJECT.md` and linked documents for technical inputs. The deployment process should be idempotent, should have scripts for backups and checking the status of services and backups."
* At this point, I will manually add CodeRabbit, Graphite > Diamond, etc., for AI code review.
* Continue development as in `Regular Development`

## Work Planning and GitHub Issues

* When the core "Hello World" works, telling me I can start planning the whole MVP, I ask: "Create GitHub issues for backend and frontend tasks, in chunks that are easy to test manually. Focus only on goals set in `specs/PROJECT.md` and linked documents. Mention `Acceptance Criteria` in each issue. Mention in issues: Types in API must be generated from the backend. Use the `gh` command for GitHub."
* You will need to use this template for MVP and then next phases.

## Regular Development

* Prompt referring to the GitHub issue: "Please attempt GitHub issue <__>, use the `gh` command, follow `Development Workflow` in `CLAUDE.md`. Follow `specs/PROJECT.md` and linked documents for technical inputs. When you want me to test, let me know; I will run the backend and web apps myself."
* You should test manually, running backend and web applications locally.
* If you have errors, see `Tackle Errors`.
* Prompt, assuming CodeRabbit, Graphite > Diamond, etc., are integrated and have commented about issues in a Pull Request: "Please read comments in PR <__> and attempt to fix them. Use the `gh` command."
* You should test manually, running backend and web applications locally; repeat for the next GitHub issue.

## Tackle Errors

* Save logs from Chrome Console or Terminal into files in a folder like `test_logs`.
* Create the folder `test_logs` in your project.
* Prompt: "I will share logs if needed during testing into `test_logs`. Please add this folder to `.gitignore`; files here should never be in git."
* Run backend, web apps, test, share errors from logs.
* Prompts like (edit as needed): "I found an error when running the web app, at URL <___>. The log is in `test_logs/chrome_console.log`."

## Tools Needed

* Terminal
* Terminal-based coding tools, usually Claude Code for initial structure.
* `git` CLI tool
* Backend language, like Python, Golang, or NodeJS.
* NodeJS for frontend development.
* GitHub CLI tool, `gh` referred to above.
* Once GitHub-based is set, a few issues work well and are tested, I use a mix of Gemini CLI, Continue.dev CLI, Qwen Code CLI, Cursor CLI.

## Development Workflow (in any agent's Markdown file)

* Create a new branch for each task
* Branch names should start with `feature/`, `chore/`, or `fix/`
* Add tests for any new features added, particularly integration or end-to-end tests
* Run formatters, linters, and tests before committing changes
* When finished, please commit and push to the new branch
* Please mention the GitHub issue if provided
* Commit small chunks
* Selectively add files to git; maintain `.gitignore`
* If working on a GitHub issue: create a PR, update the task in the end
* If working on a GitHub issue: do not close the issue until I manually test
