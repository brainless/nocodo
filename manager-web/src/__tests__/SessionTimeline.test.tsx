import { describe, expect, test } from 'vitest';
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
  ended_at: null,
};

const mockCompletedSession: AiSession = {
  id: 'session-456',
  project_id: 'project-456',
  tool_name: 'gpt-4',
  status: 'completed',
  prompt: 'Fix authentication bug',
  project_context: 'Authentication system',
  started_at: 1640995100,
  ended_at: 1640995300,
};

const mockFailedSession: AiSession = {
  id: 'session-789',
  project_id: null,
  tool_name: 'gemini',
  status: 'failed',
  prompt: 'Generate documentation',
  project_context: null,
  started_at: 1640994800,
  ended_at: 1640995000,
};

const mockCancelledSession: AiSession = {
  id: 'session-101',
  project_id: 'project-456',
  tool_name: 'claude',
  status: 'cancelled',
  prompt: 'Refactor code',
  project_context: 'Code improvement task',
  started_at: 1640994500,
  ended_at: 1640994800,
};

describe('SessionTimeline Component', () => {
  test('renders timeline header correctly', () => {
    render(() => <SessionTimeline session={mockCompletedSession} />);

    expect(screen.getByText('Session Timeline')).toBeInTheDocument();
    expect(
      screen.getByText('Track the progress and key events of this AI session')
    ).toBeInTheDocument();
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

  test('renders timeline events for failed session', () => {
    render(() => <SessionTimeline session={mockFailedSession} />);

    expect(screen.getByText('Session Created')).toBeInTheDocument();
    expect(screen.getByText('Session Started')).toBeInTheDocument();
    expect(screen.getByText('Session Failed')).toBeInTheDocument();

    // Should show error message
    expect(screen.getByText(/Ended with error after/)).toBeInTheDocument();
  });

  test('renders timeline events for cancelled session', () => {
    render(() => <SessionTimeline session={mockCancelledSession} />);

    expect(screen.getByText('Session Created')).toBeInTheDocument();
    expect(screen.getByText('Session Started')).toBeInTheDocument();
    expect(screen.getByText('Session Cancelled')).toBeInTheDocument();

    // Should show cancellation message
    expect(screen.getByText(/Cancelled by user after/)).toBeInTheDocument();
  });

  test('renders live indicator for running session', () => {
    render(() => <SessionTimeline session={mockRunningSession} />);

    expect(screen.getByText('Session Created')).toBeInTheDocument();
    expect(screen.getByText('Session Started')).toBeInTheDocument();
    expect(screen.getByText('Session Running')).toBeInTheDocument();
    expect(screen.getByText('Processing your request...')).toBeInTheDocument();
    expect(screen.getByText('Live')).toBeInTheDocument();
  });

  test('does not show live indicator for completed session', () => {
    render(() => <SessionTimeline session={mockCompletedSession} />);

    expect(screen.queryByText('Session Running')).not.toBeInTheDocument();
    expect(screen.queryByText('Live')).not.toBeInTheDocument();
  });

  test('displays timeline summary correctly', () => {
    render(() => <SessionTimeline session={mockCompletedSession} />);

    expect(screen.getByText('Total Events:')).toBeInTheDocument();
    expect(screen.getByText('Session Status:')).toBeInTheDocument();
    expect(screen.getByText('completed')).toBeInTheDocument();

    // Should show number of events (created + started + completed = 3)
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  test('renders events in chronological order', () => {
    render(() => <SessionTimeline session={mockCompletedSession} />);

    const eventTitles = screen.getAllByRole('listitem');
    // Events should be in order: created, started, completed
    expect(eventTitles[0]).toHaveTextContent('Session Created');
    expect(eventTitles[1]).toHaveTextContent('Session Started');
    expect(eventTitles[2]).toHaveTextContent('Session Completed');
  });

  test('shows proper time formatting', () => {
    render(() => <SessionTimeline session={mockCompletedSession} />);

    // Should show formatted timestamps for events
    const timeElements = screen.getAllByRole('time');
    expect(timeElements.length).toBeGreaterThan(0);

    // Each time element should have a dateTime attribute
    timeElements.forEach(timeEl => {
      expect(timeEl).toHaveAttribute('dateTime');
    });
  });

  test('calculates duration correctly in event descriptions', () => {
    render(() => <SessionTimeline session={mockCompletedSession} />);

    // For a session that ran for 200 seconds (1640995300 - 1640995100)
    expect(screen.getByText(/Finished successfully in 3m 20s/)).toBeInTheDocument();
  });

  test('applies custom className', () => {
    const { container } = render(() => (
      <SessionTimeline session={mockCompletedSession} className='custom-timeline' />
    ));

    expect(container.firstChild).toHaveClass('custom-timeline');
  });

  test('has proper accessibility attributes', () => {
    render(() => <SessionTimeline session={mockCompletedSession} />);

    // Timeline should have proper list semantics
    const timelineList = screen.getByRole('list', { name: 'Session timeline events' });
    expect(timelineList).toBeInTheDocument();

    const listItems = screen.getAllByRole('listitem');
    expect(listItems.length).toBeGreaterThan(0);
  });

  test('shows different event icons correctly', () => {
    const { container } = render(() => <SessionTimeline session={mockCompletedSession} />);

    // Should have different background colors for different event types
    const iconContainers = container.querySelectorAll('[class*="bg-"]');
    expect(iconContainers.length).toBeGreaterThan(0);
  });

  test('handles session with very short duration', () => {
    const shortSession: AiSession = {
      ...mockCompletedSession,
      started_at: 1640995200,
      ended_at: 1640995210, // 10 seconds later
    };

    render(() => <SessionTimeline session={shortSession} />);

    expect(screen.getByText(/Finished successfully in 10s/)).toBeInTheDocument();
  });

  test('handles session with no end time for running status', () => {
    render(() => <SessionTimeline session={mockRunningSession} />);

    // Should not show completion event
    expect(screen.queryByText('Session Completed')).not.toBeInTheDocument();
    expect(screen.queryByText('Session Failed')).not.toBeInTheDocument();
    expect(screen.queryByText('Session Cancelled')).not.toBeInTheDocument();

    // Should show running indicator
    expect(screen.getByText('Session Running')).toBeInTheDocument();
  });
});
