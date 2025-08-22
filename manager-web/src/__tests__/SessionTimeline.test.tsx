import { describe, test, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@solidjs/testing-library';
import SessionTimeline from '../components/SessionTimeline';
import type { AiSession } from '../types';

// Mock data
const mockRunningSession: AiSession = {
  id: 'session-123',
  project_id: 'project-456',
  tool_name: 'claude',
  status: 'running',
  prompt: 'Create a new component',
  project_context: 'React app development',
  started_at: 1640995200,
  ended_at: undefined
};

const mockCompletedSession: AiSession = {
  id: 'session-456',
  project_id: 'project-456',
  tool_name: 'gpt-4',
  status: 'completed',
  prompt: 'Fix authentication bug',
  project_context: 'Authentication system',
  started_at: 1640995100,
  ended_at: 1640995300
};

const mockFailedSession: AiSession = {
  id: 'session-789',
  project_id: undefined,
  tool_name: 'gemini',
  status: 'failed',
  prompt: 'Generate documentation',
  project_context: undefined,
  started_at: 1640994800,
  ended_at: 1640995000
};

const mockCancelledSession: AiSession = {
  id: 'session-101',
  project_id: 'project-456',
  tool_name: 'claude',
  status: 'cancelled',
  prompt: 'Refactor code',
  project_context: 'Code improvement task',
  started_at: 1640994500,
  ended_at: 1640994800
};

describe('SessionTimeline Component', () => {
  test('renders timeline header correctly', () => {
    render(() => <SessionTimeline session={mockCompletedSession} />);
    
    expect(screen.getByText('Session Timeline')).toBeInTheDocument();
    expect(screen.getByText('Track the progress and key events of this AI session')).toBeInTheDocument();
  });

  test('renders timeline events for completed session', () => {
    render(() => <SessionTimeline session={mockCompletedSession} />);
    
    // Should show created and started events
    expect(screen.getByText('Session Created')).toBeInTheDocument();
    expect(screen.getByText('Session Started')).toBeInTheDocument();
    expect(screen.getByText('Session Completed')).toBeInTheDocument();
    
    // Should show tool name in description
    expect(screen.getByText('gpt-4 session initialized')).toBeInTheDocument();
    expect(screen.getByText('Began processing with gpt-4')).toBeInTheDocument();
  });

  test('renders timeline events for failed session', () => {\n    render(() => <SessionTimeline session={mockFailedSession} />);\n    \n    expect(screen.getByText('Session Created')).toBeInTheDocument();\n    expect(screen.getByText('Session Started')).toBeInTheDocument();\n    expect(screen.getByText('Session Failed')).toBeInTheDocument();\n    \n    // Should show error message\n    expect(screen.getByText(/Ended with error after/)).toBeInTheDocument();\n  });\n\n  test('renders timeline events for cancelled session', () => {\n    render(() => <SessionTimeline session={mockCancelledSession} />);\n    \n    expect(screen.getByText('Session Created')).toBeInTheDocument();\n    expect(screen.getByText('Session Started')).toBeInTheDocument();\n    expect(screen.getByText('Session Cancelled')).toBeInTheDocument();\n    \n    // Should show cancellation message\n    expect(screen.getByText(/Cancelled by user after/)).toBeInTheDocument();\n  });\n\n  test('renders live indicator for running session', () => {\n    render(() => <SessionTimeline session={mockRunningSession} />);\n    \n    expect(screen.getByText('Session Created')).toBeInTheDocument();\n    expect(screen.getByText('Session Started')).toBeInTheDocument();\n    expect(screen.getByText('Session Running')).toBeInTheDocument();\n    expect(screen.getByText('Processing your request...')).toBeInTheDocument();\n    expect(screen.getByText('Live')).toBeInTheDocument();\n  });\n\n  test('does not show live indicator for completed session', () => {\n    render(() => <SessionTimeline session={mockCompletedSession} />);\n    \n    expect(screen.queryByText('Session Running')).not.toBeInTheDocument();\n    expect(screen.queryByText('Live')).not.toBeInTheDocument();\n  });\n\n  test('displays timeline summary correctly', () => {\n    render(() => <SessionTimeline session={mockCompletedSession} />);\n    \n    expect(screen.getByText('Total Events:')).toBeInTheDocument();\n    expect(screen.getByText('Session Status:')).toBeInTheDocument();\n    expect(screen.getByText('completed')).toBeInTheDocument();\n    \n    // Should show number of events (created + started + completed = 3)\n    expect(screen.getByText('3')).toBeInTheDocument();\n  });\n\n  test('renders events in chronological order', () => {\n    render(() => <SessionTimeline session={mockCompletedSession} />);\n    \n    const eventTitles = screen.getAllByRole('listitem');\n    // Events should be in order: created, started, completed\n    expect(eventTitles[0]).toHaveTextContent('Session Created');\n    expect(eventTitles[1]).toHaveTextContent('Session Started');\n    expect(eventTitles[2]).toHaveTextContent('Session Completed');\n  });\n\n  test('shows proper time formatting', () => {\n    render(() => <SessionTimeline session={mockCompletedSession} />);\n    \n    // Should show formatted timestamps for events\n    const timeElements = screen.getAllByRole('time');\n    expect(timeElements.length).toBeGreaterThan(0);\n    \n    // Each time element should have a dateTime attribute\n    timeElements.forEach(timeEl => {\n      expect(timeEl).toHaveAttribute('dateTime');\n    });\n  });\n\n  test('calculates duration correctly in event descriptions', () => {\n    render(() => <SessionTimeline session={mockCompletedSession} />);\n    \n    // For a session that ran for 200 seconds (1640995300 - 1640995100)\n    expect(screen.getByText(/Finished successfully in 3m 20s/)).toBeInTheDocument();\n  });\n\n  test('applies custom className', () => {\n    const { container } = render(() => (\n      <SessionTimeline session={mockCompletedSession} className=\"custom-timeline\" />\n    ));\n    \n    expect(container.firstChild).toHaveClass('custom-timeline');\n  });\n\n  test('has proper accessibility attributes', () => {\n    render(() => <SessionTimeline session={mockCompletedSession} />);\n    \n    // Timeline should have proper list semantics\n    const timelineList = screen.getByRole('list', { name: 'Session timeline events' });\n    expect(timelineList).toBeInTheDocument();\n    \n    const listItems = screen.getAllByRole('listitem');\n    expect(listItems.length).toBeGreaterThan(0);\n  });\n\n  test('shows different event icons correctly', () => {\n    const { container } = render(() => <SessionTimeline session={mockCompletedSession} />);\n    \n    // Should have different background colors for different event types\n    const iconContainers = container.querySelectorAll('[class*=\"bg-\"]');\n    expect(iconContainers.length).toBeGreaterThan(0);\n  });\n\n  test('handles session with very short duration', () => {\n    const shortSession: AiSession = {\n      ...mockCompletedSession,\n      started_at: 1640995200,\n      ended_at: 1640995210 // 10 seconds later\n    };\n    \n    render(() => <SessionTimeline session={shortSession} />);\n    \n    expect(screen.getByText(/Finished successfully in 10s/)).toBeInTheDocument();\n  });\n\n  test('handles session with no end time for running status', () => {\n    render(() => <SessionTimeline session={mockRunningSession} />);\n    \n    // Should not show completion event\n    expect(screen.queryByText('Session Completed')).not.toBeInTheDocument();\n    expect(screen.queryByText('Session Failed')).not.toBeInTheDocument();\n    expect(screen.queryByText('Session Cancelled')).not.toBeInTheDocument();\n    \n    // Should show running indicator\n    expect(screen.getByText('Session Running')).toBeInTheDocument();\n  });\n});\n"}}
</function_results>

Now let me update the existing tests to work with the new components:

<function_calls>
<invoke name="edit_files">
<parameter name="title">Update AiSessionsList tests to work with new SessionRow component
