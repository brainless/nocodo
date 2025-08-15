---
title: "Testing Fundamentals"
description: "Unit, integration, and e2e testing concepts"
module: "fundamentals"
order: 3
tags: ["testing", "unit-tests", "integration", "e2e"]
---

# Testing Fundamentals

Learn the essential testing strategies that ensure code quality and reliability in software development.

## Why Test?

- **Catch bugs early** - Before they reach production
- **Enable refactoring** - Change code with confidence
- **Document behavior** - Tests serve as living documentation
- **Improve design** - Testable code is often better designed

## Types of Testing

### Unit Tests
Test individual functions or components in isolation.

```javascript
// Example unit test
test('calculateTotal should add tax to subtotal', () => {
  const result = calculateTotal(100, 0.08);
  expect(result).toBe(108);
});
```

**Characteristics:**
- Fast execution
- No external dependencies
- High test coverage

### Integration Tests
Test how different parts of your system work together.

```javascript
// Example integration test
test('user can create and retrieve a post', async () => {
  const post = await createPost({ title: 'Test', content: 'Content' });
  const retrieved = await getPost(post.id);
  expect(retrieved.title).toBe('Test');
});
```

**Characteristics:**
- Test component interactions
- May involve databases or APIs
- Slower than unit tests

### End-to-End (E2E) Tests
Test complete user workflows from start to finish.

```javascript
// Example E2E test with Playwright
test('user can complete checkout process', async ({ page }) => {
  await page.goto('/products');
  await page.click('[data-testid="add-to-cart"]');
  await page.click('[data-testid="checkout"]');
  await expect(page.locator('.success-message')).toBeVisible();
});
```

**Characteristics:**
- Test real user scenarios
- Use actual browser/app
- Slowest but most comprehensive

## Testing Pyramid

```
     /\
    /E2E\     <- Few, slow, expensive
   /____\
  /      \
 /INTEGR.\   <- Some, medium speed
/_________\
/          \
/   UNIT    \  <- Many, fast, cheap
/____________\
```

## Test-Driven Development (TDD)

1. **Red** - Write a failing test
2. **Green** - Write minimal code to pass
3. **Refactor** - Improve code while keeping tests green

## Popular Testing Frameworks

### JavaScript/TypeScript
- **Jest** - Feature-rich testing framework
- **Vitest** - Fast Vite-native test runner
- **Playwright** - Modern E2E testing
- **Cypress** - Developer-friendly E2E testing

### Other Languages
- **Python** - pytest, unittest
- **Java** - JUnit, TestNG
- **C#** - xUnit, NUnit

## Best Practices

1. **Write tests first** - TDD or at least test-alongside
2. **Keep tests simple** - One assertion per test when possible
3. **Use descriptive names** - Tests should read like specifications
4. **Mock external dependencies** - Control test environment
5. **Maintain test coverage** - Aim for 80%+ but focus on critical paths

## Getting Started

1. Start with unit tests for core business logic
2. Add integration tests for critical workflows
3. Include E2E tests for main user journeys
4. Set up continuous testing in your CI pipeline