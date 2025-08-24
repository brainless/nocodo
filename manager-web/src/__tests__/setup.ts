// Vitest setup file
import { vi } from 'vitest';
import '@testing-library/jest-dom';

// Configure SolidJS for testing - ensure client-side environment
(globalThis as unknown as Record<string, unknown>).IS_SOLID_TEST_ENV = true;

// Ensure we have a proper DOM-like environment for SolidJS
if (!globalThis.window) {
  Object.defineProperty(globalThis, 'window', {
    value: globalThis,
    writable: true,
  });
}

if (!globalThis.document) {
  Object.defineProperty(globalThis, 'document', {
    value: globalThis.document || {},
    writable: true,
  });
}

// Mock fetch globally
global.fetch = vi.fn();

// Mock WebSocket globally
(global as Record<string, unknown>).WebSocket = vi.fn() as unknown as WebSocket;

// Mock window.location with proper URL parsing
Object.defineProperty(window, 'location', {
  value: {
    protocol: 'http:',
    host: 'localhost:3000',
    hostname: 'localhost',
    port: '3000',
    pathname: '/',
    search: '',
    hash: '',
    href: 'http://localhost:3000/',
    origin: 'http://localhost:3000'
  },
  writable: true,
});

// Mock URL constructor for SolidJS router
if (!globalThis.URL) {
  globalThis.URL = class MockURL {
    href: string;
    origin: string;
    protocol: string;
    host: string;
    pathname: string;
    search: string;
    hash: string;
    
    constructor(url: string, base?: string) {
      this.href = url;
      this.origin = 'http://localhost:3000';
      this.protocol = 'http:';
      this.host = 'localhost:3000';
      this.pathname = '/';
      this.search = '';
      this.hash = '';
    }
  } as any;
}

// Ensure History API is available
if (!globalThis.history) {
  globalThis.history = {
    pushState: vi.fn(),
    replaceState: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
    go: vi.fn(),
    state: null,
    length: 1
  } as any;
}
