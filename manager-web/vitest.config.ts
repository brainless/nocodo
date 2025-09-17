import { defineConfig } from 'vitest/config';
import solid from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solid()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/__tests__/setup.ts'],
    exclude: ['src/__tests__/e2e/**', 'src/__tests__/api-e2e/**', 'node_modules/**'],
    transformMode: {
      web: [/\.[jt]sx?$/]
    },
    server: {
      deps: {
        inline: [/solid-js/, /@solidjs\/testing-library/, /@solidjs\/router/]
      }
    },
    // Add proper handling for SolidJS
    pool: 'threads',
    poolOptions: {
      threads: {
        singleThread: true
      }
    }
  },
});
