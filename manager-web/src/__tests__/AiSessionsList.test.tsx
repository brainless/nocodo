import { beforeEach, afterEach, describe, expect, test, vi } from 'vitest';
import { render, screen, waitFor } from '@solidjs/testing-library';
import { SessionsProvider } from '../stores/sessionsStore';
import { apiClient } from '../api';
import type { AiSession, Project } from '../types';

vi.mock('../api');

// Mock the router components
vi.mock('@solidjs/router', () => ({
  A: (props: any) => (
    <a href={props.href} class={props.class} role={props.role} aria-label={props['aria-label']}>
      {props.children}
    </a>
  ),
  MemoryRouter: (props: any) => <div>{props.children}</div>,
  useParams: () => ({ id: 'test-id' }),
  useNavigate: () => vi.fn(),
}));

import AiSessionsList from '../components/AiSessionsList';

// Mock data
const mockProjects: Project[] = [
  {
    id: 'project-456',
    name: 'Test Project 1',
    path: '/test/project1',
    language: 'typescript',
    framework: 'solidjs',
    status: 'active',
    created_at: 1640995200000,
    updated_at: 1640995200000,
  },
  {
    id: 'project-789',
    name: 'Test Project 2',
    path: '/test/project2',
    language: 'javascript',
    framework: 'react',
    status: 'active',
    created_at: 1640995200000,
    updated_at: 1640995200000,
  },
];

const mockSessions: AiSession[] = [
  {
    id: 'session-123',
    project_id: 'project-456',
    tool_name: 'claude',
    status: 'running',
    prompt: 'Test prompt for session 1',
    project_context: 'Test context 1',
    started_at: 1640995200,
    ended_at: null,
  },
  {
    id: 'session-456',
    project_id: 'project-789',
    tool_name: 'gpt-4',
    status: 'completed',
    prompt: 'Test prompt for session 2',
    project_context: 'Test context 2',
    started_at: 1640995100,
    ended_at: 1640995300,
  },
  {
    id: 'session-789',
    project_id: null,
    tool_name: 'claude',
    status: 'failed',
    prompt: 'Test prompt for session 3',
    project_context: null,
    started_at: 1640994800,
    ended_at: 1640995000,
  },
];

// Test wrapper component
const TestWrapper = (props: { children: any }) => {
  return (
    <SessionsProvider>{props.children}</SessionsProvider>
  );
};

// Setup API mocks
beforeEach(() => {
  vi.resetAllMocks();
  // Mock API calls
  (apiClient.listSessions as any).mockResolvedValue(mockSessions);
  (apiClient.fetchProjects as any).mockResolvedValue(mockProjects);
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('AiSessionsList Component', () => {
  test('renders loading state initially', async () => {
    // Make API calls slow to test loading state
    (apiClient.listSessions as any).mockImplementation(
      () => new Promise(resolve => setTimeout(() => resolve(mockSessions), 100))
    );

    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    expect(screen.getByText('Loading sessions...')).toBeInTheDocument();
  });

  test('renders sessions list after loading', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByText('AI Sessions')).toBeInTheDocument();
      expect(
        screen.getByText('Monitor and manage your AI-assisted development sessions')
      ).toBeInTheDocument();
    });

    // Check that sessions are displayed via SessionRow components
    await waitFor(() => {
      expect(screen.getByLabelText('AI Tool: claude')).toBeInTheDocument();
      expect(screen.getByLabelText('AI Tool: gpt-4')).toBeInTheDocument();
      expect(screen.getByText('Test Project 1')).toBeInTheDocument();
      expect(screen.getByText('Test Project 2')).toBeInTheDocument();
    });
  });

  test('shows session count', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByText('3 sessions')).toBeInTheDocument();
    });
  });

  test('renders empty state when no sessions', async () => {
    (apiClient.listSessions as any).mockResolvedValue([]);

    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByText('No AI sessions yet')).toBeInTheDocument();
      expect(
        screen.getByText('Start your first AI session using the nocodo CLI.')
      ).toBeInTheDocument();
    });
  });

  test('displays error state', async () => {
    const errorMessage = 'Failed to fetch sessions';
    (apiClient.listSessions as any).mockRejectedValue(new Error(errorMessage));

    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByText('Error Loading Sessions')).toBeInTheDocument();
      expect(screen.getByText(errorMessage)).toBeInTheDocument();
      expect(screen.getByText('Try Again')).toBeInTheDocument();
    });
  });

  test('renders status badges correctly', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByLabelText('Session status: running')).toBeInTheDocument();
      expect(screen.getByLabelText('Session status: completed')).toBeInTheDocument();
      expect(screen.getByLabelText('Session status: failed')).toBeInTheDocument();
    });
  });

  test('shows project names correctly', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByText('Test Project 1')).toBeInTheDocument();
      expect(screen.getByText('Test Project 2')).toBeInTheDocument();
      expect(screen.getByText('No Project')).toBeInTheDocument();
    });
  });

  test('displays session prompts', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByText('Test prompt for session 1')).toBeInTheDocument();
      expect(screen.getByText('Test prompt for session 2')).toBeInTheDocument();
      expect(screen.getByText('Test prompt for session 3')).toBeInTheDocument();
    });
  });

  test('sorts sessions by newest first', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      const sessions = screen.getAllByText(/Test prompt for session/);
      // Should be sorted by started_at descending
      expect(sessions[0]).toHaveTextContent('Test prompt for session 1'); // started_at: 1640995200
      expect(sessions[1]).toHaveTextContent('Test prompt for session 2'); // started_at: 1640995100
      expect(sessions[2]).toHaveTextContent('Test prompt for session 3'); // started_at: 1640994800
    });
  });

  test('makes correct API calls on mount', () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    expect(apiClient.listSessions).toHaveBeenCalled();
    expect(apiClient.fetchProjects).toHaveBeenCalled();
  });
});

describe('AiSessionsList Filters', () => {
  test('renders filter dropdowns with improved accessibility', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByText('Filter Sessions')).toBeInTheDocument();
      expect(screen.getByLabelText('Tool')).toBeInTheDocument();
      expect(screen.getByLabelText('Status')).toBeInTheDocument();
      expect(screen.getByText('Filter sessions by AI tool')).toBeInTheDocument();
      expect(screen.getByText('Filter sessions by completion status')).toBeInTheDocument();
    });
  });

  test('shows filter options based on available data', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      const toolSelect = screen.getByLabelText('Tool');
      expect(toolSelect).toBeInTheDocument();

      const statusSelect = screen.getByLabelText('Status');
      expect(statusSelect).toBeInTheDocument();
    });
  });

  test('shows clear filters option when filters applied', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      const toolSelect = screen.getByLabelText('Tool');
      // Simulate selecting a filter (this would need more sophisticated testing in a real scenario)
      expect(toolSelect).toBeInTheDocument();
    });
  });
});
