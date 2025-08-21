// Vitest setup file
import { vi } from 'vitest';

// Mock fetch globally
global.fetch = vi.fn();

// Mock WebSocket globally  
global.WebSocket = vi.fn() as any;