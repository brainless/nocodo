// Vitest setup file
import { vi } from 'vitest';
import '@testing-library/jest-dom';

// Configure SolidJS for testing
if (typeof window !== 'undefined') {
  (globalThis as any).IS_SOLID_TEST_ENV = true;
}

// Mock fetch globally
global.fetch = vi.fn();

// Mock WebSocket globally
global.WebSocket = vi.fn() as any;

// Mock window.location
Object.defineProperty(window, 'location', {
  value: {
    protocol: 'http:',
    host: 'localhost:3000',
    href: 'http://localhost:3000',
  },
  writable: true,
});
