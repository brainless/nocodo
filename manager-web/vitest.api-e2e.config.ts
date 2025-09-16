import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node', // Use Node.js environment for API tests
    globals: true,
    include: ['src/__tests__/api-e2e/**/*.test.ts'],
    exclude: ['node_modules/**'],
    testTimeout: 30000, // Longer timeout for API tests
    retry: 2, // Retry failed tests
    pool: 'threads',
    poolOptions: {
      threads: {
        singleThread: true // Ensure tests run sequentially for server management
      }
    },
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      reportsDirectory: './coverage/api-e2e',
      include: [
        'src/__tests__/api-e2e/**/*.ts',
        'src/types/**/*.ts',
      ],
      exclude: [
        'node_modules/**',
        'src/__tests__/api-e2e/**/*.test.ts', // Exclude test files themselves
        'coverage/**',
      ],
      thresholds: {
        global: {
          branches: 70,
          functions: 80,
          lines: 80,
          statements: 80,
        },
      },
    },
    outputFile: {
      json: './test-results/api-e2e-results.json',
    },
  },
});