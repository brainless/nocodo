Add E2E Test for Project Command Discovery

Context:
- We just completed Phase 2 of project commands (see tasks/project-commands-with-llm-integration.md phases 1-2)
- Phase 2 added command discovery API: POST /api/projects/{id}/commands/discover
- Discovery detects tech stacks (Node.js, Rust, Python, Go, Java) and suggests commands

Current E2E Test:
- Script: ./run_llm_e2e_test.sh {provider} {model}
- Existing test clones Saleor repo and validates technology detection
- Located in manager/tests/ directory

Task:
1. Explore existing e2e test structure:
 - Find and read current test that validates Saleor project technology detection
 - Understand test setup, assertions, and teardown patterns
2. Update test runner script (run_llm_e2e_test.sh):
 - Change signature to: ./run_llm_e2e_test.sh {provider} {model} {test_type}
 - Default test_type to existing test if not provided
 - Support new test type: command_discovery
3. Create new e2e test:
 - Clone Saleor repo (like existing test)
 - Call POST /api/projects/{project_id}/commands/discover
 - Validate response structure:
     - Has commands array
   - Has project_types array
   - Contains install command (name contains "install")
   - Contains run/dev command (name contains "run" or "dev" or "start")
 - Assert command structure has required fields: id, name, command, description
 - Use hardcoded presets for validation (Saleor is Python/Django project)

Deliverable:
- Updated run_llm_e2e_test.sh with test type parameter
- New test file following existing e2e test patterns
- Test validates command discovery API returns install + run commands for Saleor

Keep it concise - follow existing test conventions, reuse setup/teardown code
