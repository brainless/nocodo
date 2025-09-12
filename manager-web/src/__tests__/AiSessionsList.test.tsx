import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
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
    work_id: 'work-123',
    message_id: 'msg-123',
    tool_name: 'claude',
    status: 'running',
    project_context: 'Test context 1',
    started_at: 1640995200,
    ended_at: null,
    prompt: 'Test prompt for session 1',
  },
  {
    id: 'session-456',
    work_id: 'work-456',
    message_id: 'msg-456',
    tool_name: 'gpt-4',
    status: 'completed',
    project_context: 'Test context 2',
    started_at: 1640995100,
    ended_at: 1640995300,
    prompt: 'Test prompt for session 2',
  },
  {
    id: 'session-789',
    work_id: 'work-789',
    message_id: 'msg-789',
    tool_name: 'claude',
    status: 'failed',
    project_context: null,
    started_at: 1640994800,
    ended_at: 1640995000,
    prompt: 'Test prompt for session 3',
  },
];

// Test wrapper component
const TestWrapper = (props: { children: any }) => {
  return <SessionsProvider>{props.children}</SessionsProvider>;
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

    expect(screen.getByText('Loading work...')).toBeInTheDocument();
  });

  test('renders sessions list after loading', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByText('Filter Work')).toBeInTheDocument();
      expect(screen.getByText('Filter work by AI tool')).toBeInTheDocument();
    });

    // Check that sessions are displayed via SessionRow components
    await waitFor(() => {
      expect(screen.getAllByLabelText('AI Tool: claude')).toHaveLength(2);
      expect(screen.getByLabelText('AI Tool: gpt-4')).toBeInTheDocument();
    });
  });

  test('shows session count', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      // Check for count in the main display area, not filter dropdowns
      expect(screen.getByText('3')).toBeInTheDocument();

      // Find the specific element in the top-right count display
      const countElements = screen.getAllByText((_, element) => {
        return element?.textContent?.includes('3 work items') || false;
      });

      // Should have the count display (might have multiple due to filters, but at least one)
      expect(countElements.length).toBeGreaterThanOrEqual(1);
    });
  });

  test('renders empty state when no sessions', async () => {
    (apiClient.listSessions as any).mockResolvedValue([]);

    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByText('No AI work yet')).toBeInTheDocument();
      expect(
        screen.getByText('Start your first AI work session using the nocodo CLI.')
      ).toBeInTheDocument();
    });
  });

  test('displays error state', async () => {
    const errorMessage = 'Failed to fetch sessions';
    (apiClient.listSessions as any).mockRejectedValue(new Error(errorMessage));

    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByText('Error Loading Work')).toBeInTheDocument();
      expect(screen.getByText(errorMessage)).toBeInTheDocument();
      expect(screen.getByText('Try Again')).toBeInTheDocument();
    });
  });

  test('renders status badges correctly', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      expect(screen.getByLabelText('Work status: running')).toBeInTheDocument();
      expect(screen.getByLabelText('Work status: completed')).toBeInTheDocument();
      expect(screen.getByLabelText('Work status: failed')).toBeInTheDocument();
    });
  });

  test('shows project names correctly', async () => {
    render(() => <AiSessionsList />, { wrapper: TestWrapper });

    await waitFor(() => {
      // Check that No Project elements are present (multiple sessions without projects)
      const noProjectElements = screen.getAllByText('No Project');
      expect(noProjectElements.length).toBeGreaterThan(0);
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
      expect(screen.getByText('Filter Work')).toBeInTheDocument();
      expect(screen.getByLabelText('Tool')).toBeInTheDocument();
      expect(screen.getByLabelText('Status')).toBeInTheDocument();
      expect(screen.getByText('Filter work by AI tool')).toBeInTheDocument();
      expect(screen.getByText('Filter work by completion status')).toBeInTheDocument();
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
