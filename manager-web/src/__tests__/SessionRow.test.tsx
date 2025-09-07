import { describe, expect, test, vi } from 'vitest';
import { render, screen } from '@solidjs/testing-library';
import type { AiSession, Project } from '../types';

// Mock the router components
vi.mock('@solidjs/router', () => ({
  A: (props: any) => (
    <a href={props.href} class={props.class} role={props.role} aria-label={props['aria-label']}>
      {props.children}
    </a>
  ),
  MemoryRouter: (props: any) => props.children,
  useParams: () => ({ id: 'test-id' }),
  useNavigate: () => vi.fn(),
}));

import SessionRow, { ProjectBadge, StatusBadge, ToolIcon } from '../components/SessionRow';

// Simple test wrapper
const TestWrapper = (props: { children: any }) => {
  return <div>{props.children}</div>;
};

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
  work_id: 'work-123',
  message_id: 'msg-123',
  tool_name: 'claude',
  status: 'running',
  project_context: 'Working on a React app',
  started_at: 1640995200,
  ended_at: null,
};

const mockCompletedSession: AiSession = {
  id: 'session-456',
  work_id: 'work-456',
  message_id: 'msg-456',
  tool_name: 'gpt-4',
  status: 'completed',
  project_context: 'Authentication system needs debugging',
  started_at: 1640995100,
  ended_at: 1640995300,
};

const mockSessionWithoutProject: AiSession = {
  id: 'session-789',
  work_id: 'work-789',
  message_id: 'msg-789',
  tool_name: 'gemini',
  status: 'failed',
  project_context: null,
  started_at: 1640994800,
  ended_at: 1640995000,
};

describe('StatusBadge Component', () => {
  test('renders completed status correctly', () => {
    render(() => <StatusBadge status='completed' />, { wrapper: TestWrapper });

    expect(screen.getByLabelText('Work status: completed')).toBeInTheDocument();
    expect(screen.getByText('completed')).toBeInTheDocument();
  });

  test('renders running status with animation', () => {
    render(() => <StatusBadge status='running' showIcon={true} />, { wrapper: TestWrapper });

    const badge = screen.getByLabelText('Work status: running');
    expect(badge).toBeInTheDocument();
    expect(badge.querySelector('.animate-spin')).toBeInTheDocument();
  });

  test('renders different sizes correctly', () => {
    render(
      () => (
        <div>
          <StatusBadge status='completed' size='sm' />
          <StatusBadge status='completed' size='md' />
        </div>
      ),
      { wrapper: TestWrapper }
    );

    // Both badges should be rendered
    const badges = screen.getAllByLabelText('Work status: completed');
    expect(badges).toHaveLength(2);
  });

  test('hides icon when showIcon is false', () => {
    render(() => <StatusBadge status='completed' showIcon={false} />, { wrapper: TestWrapper });

    const badge = screen.getByLabelText('Work status: completed');
    expect(badge).toBeInTheDocument();
    // Icon should not be present when showIcon is false
    expect(badge.textContent).toBe('completed');
  });
});

describe('ToolIcon Component', () => {
  test('renders Claude tool with correct icon and color', () => {
    render(() => <ToolIcon toolName='claude' />, { wrapper: TestWrapper });

    const toolIcon = screen.getByLabelText('AI Tool: claude');
    expect(toolIcon).toBeInTheDocument();
    expect(toolIcon).toHaveClass('bg-orange-100', 'text-orange-800');
    expect(toolIcon.textContent).toContain('claude');
  });

  test('renders GPT tool with correct styling', () => {
    render(() => <ToolIcon toolName='gpt-4' />, { wrapper: TestWrapper });

    const toolIcon = screen.getByLabelText('AI Tool: gpt-4');
    expect(toolIcon).toBeInTheDocument();
    expect(toolIcon).toHaveClass('bg-green-100', 'text-green-800');
  });

  test('renders Gemini tool with correct styling', () => {
    render(() => <ToolIcon toolName='gemini' />, { wrapper: TestWrapper });

    const toolIcon = screen.getByLabelText('AI Tool: gemini');
    expect(toolIcon).toBeInTheDocument();
    expect(toolIcon).toHaveClass('bg-purple-100', 'text-purple-800');
  });

  test('renders default styling for unknown tools', () => {
    render(() => <ToolIcon toolName='unknown-tool' />, { wrapper: TestWrapper });

    const toolIcon = screen.getByLabelText('AI Tool: unknown-tool');
    expect(toolIcon).toBeInTheDocument();
    expect(toolIcon).toHaveClass('bg-gray-100', 'text-gray-800');
  });
});

describe('ProjectBadge Component', () => {
  test('renders project with name correctly', () => {
    render(() => <ProjectBadge project={mockProject} />, { wrapper: TestWrapper });

    expect(screen.getByText('Test Project')).toBeInTheDocument();
  });

  test('renders no project state', () => {
    render(() => <ProjectBadge project={null} />, { wrapper: TestWrapper });

    expect(screen.getByText('No Project')).toBeInTheDocument();
  });

  test('renders project ID when no project object', () => {
    render(() => <ProjectBadge project={null} projectId='project-123' />, { wrapper: TestWrapper });

    expect(screen.getByText('Project project-123')).toBeInTheDocument();
  });
});

describe('SessionRow Component', () => {
  test('renders session information correctly', () => {
    render(
      () => <SessionRow session={mockRunningSession} project={mockProject} showPrompt={true} />,
      { wrapper: TestWrapper }
    );

    // Check session details
    expect(screen.getByText('claude')).toBeInTheDocument();
    expect(screen.getByText('Test Project')).toBeInTheDocument();
    expect(screen.getByText('Create a new React component')).toBeInTheDocument();
    expect(screen.getByLabelText('Work status: running')).toBeInTheDocument();
  });

  test('renders completed session correctly', () => {
    render(
      () => <SessionRow session={mockCompletedSession} project={mockProject} showPrompt={true} />,
      { wrapper: TestWrapper }
    );

    expect(screen.getByText('gpt-4')).toBeInTheDocument();
    expect(screen.getByText('Fix bug in authentication')).toBeInTheDocument();
    expect(screen.getByLabelText('Work status: completed')).toBeInTheDocument();

    // Should not show running indicator for completed sessions
    expect(screen.queryByText('Work is actively running')).not.toBeInTheDocument();
  });

  test('renders session without project', () => {
    render(
      () => <SessionRow session={mockSessionWithoutProject} project={null} showPrompt={true} />,
      { wrapper: TestWrapper }
    );

    expect(screen.getByText('gemini')).toBeInTheDocument();
    expect(screen.getByText('No Project')).toBeInTheDocument();
    expect(screen.getByText('Generate documentation')).toBeInTheDocument();
  });

  test('hides prompt when showPrompt is false', () => {
    render(
      () => <SessionRow session={mockRunningSession} project={mockProject} showPrompt={false} />,
      { wrapper: TestWrapper }
    );

    // Session details should be visible
    expect(screen.getByText('claude')).toBeInTheDocument();
    expect(screen.getByText('Test Project')).toBeInTheDocument();

    // Prompt should not be visible
    expect(screen.queryByText('Create a new React component')).not.toBeInTheDocument();
  });

  test('shows running indicator for active sessions', () => {
    render(
      () => <SessionRow session={mockRunningSession} project={mockProject} showPrompt={true} />,
      { wrapper: TestWrapper }
    );

    expect(screen.getByText('Work is actively running')).toBeInTheDocument();
  });

  test('shows duration with ongoing indicator for running sessions', () => {
    render(
      () => <SessionRow session={mockRunningSession} project={mockProject} showPrompt={true} />,
      { wrapper: TestWrapper }
    );

    expect(screen.getByText(/(ongoing)/)).toBeInTheDocument();
  });

  test('creates correct link to session detail', () => {
    render(
      () => <SessionRow session={mockRunningSession} project={mockProject} showPrompt={true} />,
      { wrapper: TestWrapper }
    );

    const link = screen.getByRole('article');
    expect(link).toHaveAttribute('href', '/work/session-123');
  });

  test('has proper accessibility attributes', () => {
    render(
      () => <SessionRow session={mockRunningSession} project={mockProject} showPrompt={true} />,
      { wrapper: TestWrapper }
    );

    const link = screen.getByRole('article');
    expect(link).toHaveAttribute('aria-label', 'View details for claude work');

    // Check for time elements with proper datetime attributes
    const timeElement = screen.getByRole('time');
    expect(timeElement).toHaveAttribute('dateTime');
  });

  test('applies custom className', () => {
    render(
      () => (
        <SessionRow
          session={mockRunningSession}
          project={mockProject}
          showPrompt={true}
          className='custom-class'
        />
      ),
      { wrapper: TestWrapper }
    );

    const listItem = screen.getByRole('article').closest('li');
    expect(listItem).toHaveClass('custom-class');
  });
});
