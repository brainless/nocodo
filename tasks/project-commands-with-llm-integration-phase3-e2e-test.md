# Add E2E Test for LLM-Enhanced Command Discovery (Phase 3)

## Context
- Phase 3 LLM integration is complete (see tasks/project-commands-with-llm-integration.md)
- Existing test validates rule-based discovery only (Phase 2)
- Need test that validates actual LLM enhancement of discovered commands

## Current State
- Existing test: `test_command_discovery_saleor` (tests rule-based only)
- Handler supports `?use_llm=true/false` query parameter
- LLM enhancement falls back gracefully when unavailable

## Task
Create new E2E test that validates LLM-enhanced command discovery:

1. **Setup**:
   - Reuse existing test infrastructure (TestApp, Saleor project)
   - Configure LLM credentials (use env vars: ANTHROPIC_API_KEY, PROVIDER, MODEL)
   - Skip test if no API key configured

2. **Test Execution**:
   - Call: `POST /api/projects/{id}/commands/discover?use_llm=true`
   - Wait for LLM response (may take 10-30 seconds)

3. **Validation**:
   - Assert `llm_used: true` in response
   - Assert `reasoning` field contains LLM explanation
   - Assert commands have enhanced descriptions (not just "Run X script")
   - Assert LLM may add/remove commands vs rule-based
   - Compare with rule-based results (call with `?use_llm=false`)

4. **Test Name**: `test_command_discovery_llm_enhanced_saleor`

## Deliverable
- New test in `manager/tests/llm_e2e_command_discovery_test.rs`
- Test skipped if `ANTHROPIC_API_KEY` not set (use `#[ignore]` or conditional skip)
- Documented in test comments: "Requires ANTHROPIC_API_KEY env var"

## Reference
- Existing test: `test_command_discovery_saleor` (rule-based)
- Handler: `manager/src/handlers/project_commands.rs::discover_project_commands`
- Design doc: `tasks/project-commands-with-llm-integration.md` Phase 3
