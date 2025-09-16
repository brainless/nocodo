# API-Only E2E Tests Migration Guide

This guide helps you migrate from browser-based Playwright tests to fast, reliable API-only end-to-end tests.

## 🎯 Migration Overview

### Why Migrate?
- **10-20x faster execution** - No browser startup/teardown
- **More reliable** - No UI timing issues or DOM dependencies
- **Better coverage** - Direct API testing without UI abstraction
- **Easier debugging** - Clear request/response logging
- **CI/CD friendly** - No headless browser requirements

### Before vs After

| Aspect | Browser-Based (Playwright) | API-Only (Vitest) |
|--------|---------------------------|-------------------|
| **Speed** | ~30-60 seconds | ~5-10 seconds |
| **Reliability** | Flaky (DOM, timing) | Stable (direct API) |
| **Debugging** | Screenshots, DOM inspection | Request/response logs |
| **Setup** | Browser, headless mode | Node.js environment |
| **Coverage** | UI behavior | API contracts |
| **Maintenance** | High (UI changes) | Low (API changes) |

## 🚀 Quick Start Migration

### 1. Update Test Scripts

Replace Playwright commands with API test commands:

```bash
# Old commands
npm run test:e2e          # Playwright browser tests
npm run test:e2e:ui       # Playwright with UI

# New commands
npm run test:api-e2e      # API-only tests
npm run test:api-e2e:watch # API tests in watch mode
npm run test:api-e2e:coverage # With coverage
```

### 2. Test Structure Comparison

#### Old Structure (Browser-Based)
```typescript
// playwright/e2e/agent-integration.test.ts
test.describe('Agent Integration', () => {
  test('should process file listing request', async ({ page }) => {
    // Navigate to UI
    await page.goto('/');

    // Interact with DOM elements
    await page.fill('textarea#prompt', 'List files');
    await page.click('button[type="submit"]');

    // Wait for UI updates
    await page.waitForSelector('.response-content');
    const content = await page.textContent('.response-content');

    // Assert on UI state
    expect(content).toContain('file');
  });
});
```

#### New Structure (API-Only)
```typescript
// api-e2e/workflows/llm-agent.test.ts
describe('LLM Agent Integration', () => {
  it('should process file listing tool call', async () => {
    // Create work session
    const work = await testApiClient.createWork(workData);

    // Add user message
    await testApiClient.addMessageToWork(work.work.id, {
      content: 'List all files',
      author_type: 'user'
    });

    // Create AI session
    const aiSession = await testApiClient.createAiSession(work.work.id, sessionData);

    // Execute tool call directly
    const fileList = await testApiClient.listFiles({ project_id: projectId });

    // Record AI output
    await testApiClient.recordAiOutput(work.work.id, `Found ${fileList.files.length} files`);

    // Assert on API results
    expect(fileList.files.length).toBeGreaterThan(0);
  });
});
```

## 📋 Migration Steps

### Step 1: Identify Test Categories

Categorize your existing tests:

1. **UI-Specific Tests** → Keep with Playwright
   - Visual regression tests
   - CSS/styling tests
   - Accessibility tests
   - User interaction flows

2. **API Contract Tests** → Migrate to API-only
   - Data validation
   - Business logic
   - Integration flows
   - Error handling

3. **End-to-End Workflows** → Split and migrate
   - Break down into API + UI components
   - Test API logic separately
   - Test UI interactions separately

### Step 2: Set Up Test Infrastructure

#### API Client Setup
```typescript
// Use the provided test API client
import { testApiClient } from '../setup/api-client';

// For custom clients
const customClient = new TestApiClient('http://localhost:8081');
```

#### Test Data Management
```typescript
// Use test data generators
import { testDataGenerator } from '../setup/test-data';

const project = testDataGenerator.generateProjectData({
  name: 'Test Project',
  language: 'rust'
});
```

#### State Management Testing
```typescript
// Use state manager for SolidJS integration tests
import { testStateManager } from '../utils/state-manager';

const project = await testStateManager.addProject(projectData);
const work = await testStateManager.addWorkSession(workData);
```

### Step 3: Convert Test Patterns

#### Pattern 1: Form Submission → API Call

**Before:**
```typescript
await page.fill('input[name="projectName"]', 'My Project');
await page.selectOption('select[name="language"]', 'rust');
await page.click('button[type="submit"]');
await page.waitForURL('**/projects/*');
```

**After:**
```typescript
const project = await testApiClient.createProject({
  name: 'My Project',
  language: 'rust'
});
expect(project.id).toBeDefined();
```

#### Pattern 2: UI State Verification → API Response Validation

**Before:**
```typescript
await expect(page.locator('.project-name')).toHaveText('My Project');
await expect(page.locator('.project-language')).toHaveText('rust');
```

**After:**
```typescript
expect(project.name).toBe('My Project');
expect(project.language).toBe('rust');
```

#### Pattern 3: Complex User Flows → Step-by-Step API Calls

**Before:**
```typescript
// Complex multi-step UI workflow
await page.goto('/projects/new');
await page.fill('#project-name', 'Complex Project');
// ... many UI interactions
await page.waitForSelector('.success-message');
```

**After:**
```typescript
// Direct API workflow testing
const project = await testApiClient.createProject(projectData);
const work = await testApiClient.createWork(workData);
const aiSession = await testApiClient.createAiSession(work.work.id, sessionData);
// ... direct API calls
expect(project.id).toBeDefined();
expect(work.work.id).toBeDefined();
```

### Step 4: Handle Asynchronous Operations

#### WebSocket Events → Direct API Calls

**Before:**
```typescript
// Wait for WebSocket event in UI
await page.waitForSelector('.llm-response');
// Complex timing-dependent assertions
```

**After:**
```typescript
// Direct API testing with predictable timing
await testApiClient.recordAiOutput(workId, 'AI response');
const outputs = await testApiClient.listAiOutputs(workId);
expect(outputs.outputs).toContain('AI response');
```

#### Polling Operations → Direct State Checks

**Before:**
```typescript
// Poll UI for status changes
await page.waitForSelector('.status-completed');
```

**After:**
```typescript
// Direct status checking
const work = await testApiClient.getWork(workId);
expect(work.work.status).toBe('completed');
```

### Step 5: Error Handling Migration

#### UI Error Messages → API Error Responses

**Before:**
```typescript
await page.fill('#project-name', '');
await page.click('button[type="submit"]');
await expect(page.locator('.error-message')).toBeVisible();
```

**After:**
```typescript
await expect(testApiClient.createProject({
  name: '', // Invalid
  language: 'rust'
})).rejects.toThrow();
```

### Step 6: Performance Testing

#### UI Performance → API Performance

**Before:**
```typescript
const startTime = Date.now();
// UI operations...
const endTime = Date.now();
// Rough performance measurement
```

**After:**
```typescript
// Precise API performance measurement
const timings: number[] = [];
for (let i = 0; i < 100; i++) {
  const startTime = performance.now();
  await testApiClient.createProject(projectData);
  const endTime = performance.now();
  timings.push(endTime - startTime);
}
const avgTime = timings.reduce((a, b) => a + b, 0) / timings.length;
expect(avgTime).toBeLessThan(500); // 500ms target
```

## 🧪 Test Categories Migration

### 1. Project Management Tests

**Keep UI Tests For:**
- Project creation form validation
- Project listing UI/UX
- Navigation flows

**Migrate to API Tests:**
- Project CRUD operations
- Project validation logic
- Project relationships

### 2. File Operation Tests

**Keep UI Tests For:**
- File upload UI
- Drag & drop interactions
- File browser UI

**Migrate to API Tests:**
- File CRUD operations
- File content validation
- Directory operations

### 3. LLM Agent Tests

**Keep UI Tests For:**
- Chat interface interactions
- Real-time UI updates
- User experience flows

**Migrate to API Tests:**
- Tool call processing
- AI session management
- Message handling logic

### 4. Integration Tests

**Keep UI Tests For:**
- Full user journeys
- Cross-component interactions
- Visual workflows

**Migrate to API Tests:**
- Business logic workflows
- Data flow validation
- System integration

## 🔧 Configuration Changes

### Vitest Configuration

Create `vitest.api-e2e.config.ts`:

```typescript
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    globals: true,
    include: ['src/__tests__/api-e2e/**/*.test.ts'],
    testTimeout: 30000,
    retry: 2,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      reportsDirectory: './coverage/api-e2e',
    },
  },
});
```

### Package.json Scripts

Add to `package.json`:

```json
{
  "scripts": {
    "test:api-e2e": "vitest run --config vitest.api-e2e.config.ts",
    "test:api-e2e:watch": "vitest --config vitest.api-e2e.config.ts",
    "test:api-e2e:coverage": "vitest run --config vitest.api-e2e.config.ts --coverage"
  }
}
```

## 📊 Measuring Success

### Performance Metrics
- **Target**: 10-20x faster than browser tests
- **Reliability**: < 5% flaky tests (vs 15-20% for UI tests)
- **Coverage**: > 80% API endpoint coverage

### Quality Metrics
- **Debugging**: Clear error messages with request/response logs
- **Maintenance**: Easy to update when APIs change
- **CI/CD**: Reliable in automated environments

## 🚨 Common Migration Issues

### Issue 1: Missing Test Data
**Problem**: Tests fail due to missing setup data
**Solution**: Use `testDataGenerator` for consistent test fixtures

### Issue 2: Timing Dependencies
**Problem**: Tests expect specific timing behavior
**Solution**: Use direct API calls with predictable timing

### Issue 3: UI State Assumptions
**Problem**: Tests assume UI state that doesn't exist in API tests
**Solution**: Focus on data state rather than UI state

### Issue 4: WebSocket Testing
**Problem**: Real-time features hard to test without UI
**Solution**: Use WebSocket client utilities for direct WebSocket testing

## 🎯 Best Practices

### 1. Test Organization
- Group tests by functionality (workflows, integration, performance)
- Use descriptive test names
- Keep tests focused and fast

### 2. Data Management
- Use test data generators for consistency
- Clean up test data between runs
- Avoid test data dependencies

### 3. Error Handling
- Test both success and failure scenarios
- Use proper assertions for error cases
- Test edge cases and boundary conditions

### 4. Performance
- Set reasonable performance expectations
- Monitor for regressions
- Use statistical analysis for performance tests

## 📈 Next Steps

1. **Start Small**: Migrate one test category at a time
2. **Measure Impact**: Track performance and reliability improvements
3. **Iterate**: Refine test patterns based on experience
4. **Expand Coverage**: Add more API test scenarios
5. **Deprecate UI Tests**: Gradually reduce browser-based test reliance

## 🆘 Getting Help

- Check existing API test examples in `src/__tests__/api-e2e/`
- Review test utilities in `src/__tests__/api-e2e/setup/`
- Run tests with `--reporter=verbose` for detailed output
- Check coverage reports for untested code paths

## 📚 Additional Resources

- [Vitest Documentation](https://vitest.dev/)
- [API Testing Best Practices](https://swagger.io/resources/articles/best-practices-in-api-testing/)
- [Test-Driven Development](https://martinfowler.com/bliki/TestDrivenDevelopment.html)