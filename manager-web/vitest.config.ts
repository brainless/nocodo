import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/__tests__/setup.ts'],
    transformMode: {
      web: [/\.[jt]sx?$/]
    },
    server: {
      deps: {
        inline: [/solid-js/, /@solidjs\/testing-library/]
      }
    }
  },
});
