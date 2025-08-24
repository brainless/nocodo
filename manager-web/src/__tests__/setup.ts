// Vitest setup file
import { vi } from 'vitest';
import '@testing-library/jest-dom';

// Configure SolidJS for testing - ensure client-side environment
(globalThis as unknown as Record<string, unknown>).IS_SOLID_TEST_ENV = true;

// Ensure we have a DOM-like environment for SolidJS
Object.defineProperty(globalThis, 'window', {
  value: globalThis,
  writable: true,
});

Object.defineProperty(globalThis, 'document', {
  value: globalThis.document || {},
  writable: true,
});

// Mock fetch globally
global.fetch = vi.fn();

// Mock WebSocket globally
(global as Record<string, unknown>).WebSocket = vi.fn() as unknown as WebSocket;

// Mock window.location
Object.defineProperty(window, 'location', {
  value: {
    protocol: 'http:',
    host: 'localhost:3000',
    href: 'http://localhost:3000',
  },
  writable: true,
});
