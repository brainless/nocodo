import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import type { Component } from 'solid-js';
import AiSessionDetail from '../components/AiSessionDetail';
import { SessionsProvider } from '../stores/sessionsStore';
import { apiClient } from '../api';
import type { AiSession, Project } from '../types';

vi.mock('../api');

// Mock data
const mockProject: Project = {
  id: 'project-456',
  name: 'Test Project',
  path: '/test/project',
  language: 'typescript',
  framework: 'solidjs',
  status: 'active',
  created_at: 1640995200000,
  updated_at: 1640995200000,
};

const mockRunningSession: AiSession = {
  id: 'session-123',
  project_id: 'project-456',
  tool_name: 'claude',
  status: 'running',
  prompt: 'This is a test prompt for the AI session',
  project_context: 'Test project context with some details',
  started_at: 1640995200,
  ended_at: null,
};

const mockCompletedSession: AiSession = {
  id: 'session-456',
  project_id: 'project-456',
  tool_name: 'gpt-4',
  status: 'completed',
  prompt: 'This is a completed session prompt',
  project_context: 'Completed session context',
  started_at: 1640995100,
  ended_at: 1640995300,
};

const mockSessionWithoutProject: AiSession = {
  id: 'session-789',
  project_id: null,
  tool_name: 'claude',
  status: 'failed',
  prompt: 'Session without project',
  project_context: null,
  started_at: 1640994800,
  ended_at: 1640995000,
};

// Test wrapper component with router
const TestWrapper: Component<{ sessionId?: string }> = _props => {
  return (
    <SessionsProvider>
      <AiSessionDetail />
    </SessionsProvider>
  );
};

beforeEach(() => {
  vi.resetAllMocks();

  // Mock API calls
  (apiClient.getSession as any).mockResolvedValue(mockRunningSession);
  (apiClient.fetchProject as any).mockResolvedValue(mockProject);
  (apiClient.subscribeSession as any).mockReturnValue({ close: vi.fn() });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe.skip('AiSessionDetail Component', () => {
  test('renders loading state initially', async () => {
    // Make API call slow to test loading state
    (apiClient.getSession as any).mockImplementation(
      () => new Promise(resolve => setTimeout(() => resolve(mockRunningSession), 100))
    );

    render(() => <TestWrapper />);

    expect(screen.getByText('Loading session...')).toBeInTheDocument();
  });

  test('renders session details after loading', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      expect(screen.getByText('Session Details')).toBeInTheDocument();
      expect(screen.getByText('Session ID: session-123')).toBeInTheDocument();
    });

    // Check session information
    await waitFor(() => {
      expect(screen.getByText('claude')).toBeInTheDocument();
      expect(screen.getByText('Test Project')).toBeInTheDocument();
      expect(screen.getByText('running')).toBeInTheDocument();
    });
  });

  test('displays error state', async () => {
    const errorMessage = 'Failed to fetch session';
    (apiClient.getSession as any).mockRejectedValue(new Error(errorMessage));

    render(() => <TestWrapper />);

    await waitFor(() => {
      expect(screen.getByText('Error')).toBeInTheDocument();
      expect(screen.getByText(errorMessage)).toBeInTheDocument();
    });
  });

  test('shows session not found state', async () => {
    (apiClient.getSession as any).mockResolvedValue(null);

    render(() => <TestWrapper sessionId='nonexistent' />);

    await waitFor(() => {
      expect(screen.getByText('Session not found')).toBeInTheDocument();
      expect(screen.getByText('← Back to sessions')).toBeInTheDocument();
    });
  });

  test('renders all session information fields', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      // Check all the information sections
      expect(screen.getByText('Tool')).toBeInTheDocument();
      expect(screen.getByText('Project')).toBeInTheDocument();
      expect(screen.getByText('Started')).toBeInTheDocument();
      expect(screen.getByText('Duration')).toBeInTheDocument();
      expect(screen.getByText('Current Status')).toBeInTheDocument();
      expect(screen.getByText('Prompt')).toBeInTheDocument();
      expect(screen.getByText('Project Context')).toBeInTheDocument();
    });
  });

  test('displays prompt and context correctly', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      expect(screen.getByText('This is a test prompt for the AI session')).toBeInTheDocument();
      expect(screen.getByText('Test project context with some details')).toBeInTheDocument();
    });
  });

  test('shows live status indicator for running sessions', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      expect(screen.getByText('Live updates')).toBeInTheDocument();
    });
  });

  test('does not show live indicator for completed sessions', async () => {
    (apiClient.getSession as any).mockResolvedValue(mockCompletedSession);

    render(() => <TestWrapper sessionId='session-456' />);

    await waitFor(() => {
      expect(screen.getByText('completed')).toBeInTheDocument();
    });

    // Should not show live updates for completed sessions
    expect(screen.queryByText('Live updates')).not.toBeInTheDocument();
  });

  test('handles session without project', async () => {
    (apiClient.getSession as any).mockResolvedValue(mockSessionWithoutProject);

    render(() => <TestWrapper sessionId='session-789' />);

    await waitFor(() => {
      expect(screen.getByText('No Project')).toBeInTheDocument();
      expect(screen.queryByText('Project Context')).not.toBeInTheDocument(); // Should not show context section
    });
  });

  test('shows ended timestamp for completed sessions', async () => {
    (apiClient.getSession as any).mockResolvedValue(mockCompletedSession);

    render(() => <TestWrapper sessionId='session-456' />);

    await waitFor(() => {
      expect(screen.getByText('Ended')).toBeInTheDocument();
    });
  });

  test('does not show ended timestamp for running sessions', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      expect(screen.getByText('running')).toBeInTheDocument();
    });

    expect(screen.queryByText('Ended')).not.toBeInTheDocument();
  });

  test('shows duration with ongoing indicator for running sessions', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      expect(screen.getByText('(ongoing)')).toBeInTheDocument();
    });
  });

  test('renders status badge with correct styling', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      const statusBadge = screen.getByText('running');
      expect(statusBadge).toBeInTheDocument();
      expect(statusBadge.parentElement).toHaveClass('bg-blue-100', 'text-blue-800');
    });
  });

  test('renders project link when project is available', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      const projectLink = screen.getByRole('link', { name: 'Test Project' });
      expect(projectLink).toBeInTheDocument();
      expect(projectLink).toHaveAttribute('href', '/projects/project-456/files');
    });
  });

  test('renders breadcrumb navigation', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      expect(screen.getByText('AI Sessions')).toBeInTheDocument();
      expect(screen.getByText('Session Details')).toBeInTheDocument();
      expect(screen.getByText('›')).toBeInTheDocument();
    });
  });

  test('renders back to sessions link', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      const backLink = screen.getByRole('link', { name: '← Back to Sessions' });
      expect(backLink).toBeInTheDocument();
      expect(backLink).toHaveAttribute('href', '/ai/sessions');
    });
  });

  test('makes correct API calls on mount', async () => {
    render(() => <TestWrapper />);

    // Should call getSession and fetchProject
    await waitFor(() => {
      expect(apiClient.fetchProject).toHaveBeenCalledWith('project-456');
    });
  });

  test('subscribes to live updates for running sessions', async () => {
    render(() => <TestWrapper />);

    await waitFor(() => {
      expect(apiClient.subscribeSession).toHaveBeenCalledWith(
        'session-123',
        expect.any(Function),
        expect.any(Function),
        expect.any(Function),
        expect.any(Function)
      );
    });
  });
});
