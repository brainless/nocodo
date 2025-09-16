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
    }
  },
});