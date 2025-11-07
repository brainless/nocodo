# Roadmap for nocodo

## Here is the high level roadmap of nocodo for the next 12-16 weeks.

### Users and Teams
- Desktop app > Page (left sidebar > Users, before "Servers" items) to see list of users
- Users page should have a table with Select (checkbox per row), ID, username, email, comma separated team names for the user
- We need a search bar in Users page before the table (search by username, email)
- Clicking on a user row should open a modal with same details as in the table but with editable fields
- In the User detail modal, team list should be checkboxes (show all tables and select the ones the user is part of)
- "Update" and "Cancel" buttons in the modal and should update the data or cancel the modal - doing either should refresh the table (with the search filters as they are)

### Teams and Permissions
- Desktop app > "Team" page (left sidebar, after "Users" item)
- Teams page should have a table with ID,
- Teams can be given permissions

### Prompt Library
- A list of prompts with context (like a description) and labels to help search/manage them
- manager will have new table `prompt_library` to store prompts
- Desktop app will have a `Prompt Library` in left sidebar - a page listing all prompts in the library
- Desktop app > Prompt Library page will have a top search bar to search prompt or context and check boxes for labels
- Prompts can be used to create new Work in desktop app (fills the Work create form with the selected prompt)
- Prompts from Library can be added to existing Work (details needed)

### Local Commands
- Commands like "Run full-stack", "Run backend", "Restart {project}", "Test" will be saved by manager
- manager will use specific prompts to ask AI for commands related to project and save the command, working directory, environment variables, setup commands, teardown commands, etc.
- There will also be commands to setup test DB, cache or other infra

### Evaluations
- A Prompt can ask for evaluation
- Evaluation is a check that AI response has to pass

### Hooks and Prompt Automation
- manager will define hooks: `list_files`, `read_file`, `write_file`, etc.
- Prompt Automation defines prompt (from Library) and hook when the prompt gets injected into Work
- Prompt Automation can have evaluations

### Project Context
- Project context is the tech stack, tooling, dependencies or other such details of a project
- manager will use specific prompts from Library to maintain project context that can be injected into Work

### Test deployment
- manager will execute "Test full-stack" or similar commands to deploy test setup

### Git based workflow
- manager can create and manage git worktree per Work
- At the project level, admin can enfore GitHub PR based workflow
- API endpoints in manager to create git worktree for new Work
- On completion of a Work, manager will check git commit or request commits
- For manual review of Work, manager will create PR if it is configured for Project
- Otherwise manager will wait for manual testing and then merge to main
