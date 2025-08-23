import { Component, For, Show, createMemo } from 'solid-js';
import { AiSession, AiSessionStatus } from '../types';

// Timeline event interface
interface TimelineEvent {
  id: string;
  type: 'created' | 'started' | 'completed' | 'failed' | 'cancelled';
  timestamp: number;
  title: string;
  description?: string;
  icon: string;
  iconColor: string;
}

// Utility function to format timestamp for timeline
const formatTimelineTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp * 1000);
  return date.toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  });
};

// Generate timeline events from session data
const generateTimelineEvents = (session: AiSession): TimelineEvent[] => {
  const events: TimelineEvent[] = [];

  // Session created event
  events.push({
    id: 'created',
    type: 'created',
    timestamp: session.started_at,
    title: 'Session Created',
    description: `${session.tool_name} session initialized`,
    icon: '🚀',
    iconColor: 'bg-blue-500'
  });

  // Session started event (for now, same as created)
  events.push({
    id: 'started',
    type: 'started',
    timestamp: session.started_at,
    title: 'Session Started',
    description: `Began processing with ${session.tool_name}`,
    icon: '▶️',
    iconColor: 'bg-green-500'
  });

  // Session end event (if completed)
  if (session.ended_at) {
    const duration = Math.floor((session.ended_at - session.started_at));
    const durationText = duration > 60 ? `${Math.floor(duration / 60)}m ${duration % 60}s` : `${duration}s`;

    switch (session.status) {
      case 'completed':
        events.push({
          id: 'completed',
          type: 'completed',
          timestamp: session.ended_at,
          title: 'Session Completed',
          description: `Finished successfully in ${durationText}`,
          icon: '✅',
          iconColor: 'bg-green-500'
        });
        break;
      case 'failed':
        events.push({
          id: 'failed',
          type: 'failed',
          timestamp: session.ended_at,
          title: 'Session Failed',
          description: `Ended with error after ${durationText}`,
          icon: '❌',
          iconColor: 'bg-red-500'
        });
        break;
      case 'cancelled':
        events.push({
          id: 'cancelled',
          type: 'cancelled',
          timestamp: session.ended_at,
          title: 'Session Cancelled',
          description: `Cancelled by user after ${durationText}`,
          icon: '⚪',
          iconColor: 'bg-gray-500'
        });
        break;
    }
  }

  return events.sort((a, b) => a.timestamp - b.timestamp);
};

// Individual timeline event component
const TimelineEventItem: Component<{ 
  event: TimelineEvent; 
  isLast: boolean;
  isActive?: boolean;
}> = (props) => {
  return (
    <div class="relative flex items-start">
      {/* Connector line */}
      <Show when={!props.isLast}>
        <div class="absolute top-8 left-4 w-0.5 h-full bg-gray-200" aria-hidden="true"></div>
      </Show>

      {/* Event icon */}
      <div class={`relative flex items-center justify-center w-8 h-8 rounded-full text-white text-sm font-medium ${props.event.iconColor} ${props.isActive ? 'ring-4 ring-blue-100' : ''}`}>
        <span aria-hidden="true">{props.event.icon}</span>
      </div>

      {/* Event content */}
      <div class="ml-4 flex-1 min-w-0">
        <div class="flex items-center justify-between">
          <h4 class="text-sm font-medium text-gray-900">{props.event.title}</h4>
          <time 
            class="text-xs text-gray-500"
            dateTime={new Date(props.event.timestamp * 1000).toISOString()}
            title={new Date(props.event.timestamp * 1000).toLocaleString()}
          >
            {formatTimelineTimestamp(props.event.timestamp)}
          </time>
        </div>
        <Show when={props.event.description}>
          <p class="text-sm text-gray-600 mt-1">{props.event.description}</p>
        </Show>
      </div>
    </div>
  );
};

// Live progress indicator for running sessions
const LiveProgressIndicator: Component<{ session: AiSession }> = (props) => {
  const session = () => props.session;
  
  if (session().status !== 'running') return null;

  return (
    <div class="relative flex items-start">
      {/* Animated connector line */}
      <div class="absolute top-8 left-4 w-0.5 h-8 bg-gradient-to-b from-blue-500 to-transparent animate-pulse" aria-hidden="true"></div>

      {/* Live indicator */}
      <div class="relative flex items-center justify-center w-8 h-8 rounded-full bg-blue-500 ring-4 ring-blue-100 animate-pulse">
        <div class="w-3 h-3 rounded-full bg-white animate-ping"></div>
      </div>

      {/* Live content */}
      <div class="ml-4 flex-1 min-w-0">
        <div class="flex items-center justify-between">
          <h4 class="text-sm font-medium text-blue-900">Session Running</h4>
          <div class="flex items-center text-xs text-blue-600">
            <div class="w-2 h-2 bg-blue-400 rounded-full animate-pulse mr-2" aria-hidden="true"></div>
            <span>Live</span>
          </div>
        </div>
        <p class="text-sm text-blue-700 mt-1">Processing your request...</p>
      </div>
    </div>
  );
};

// Props interface for SessionTimeline
interface SessionTimelineProps {
  session: AiSession;
  className?: string;
}

// Main SessionTimeline component
const SessionTimeline: Component<SessionTimelineProps> = (props) => {
  const session = () => props.session;
  
  const timelineEvents = createMemo(() => {
    return generateTimelineEvents(session());
  });

  const isSessionRunning = createMemo(() => {
    return session().status === 'running';
  });

  return (
    <div class={`bg-white rounded-lg border border-gray-200 p-6 ${props.className || ''}`}>
      {/* Timeline header */}
      <div class="mb-6">
        <h3 class="text-lg font-medium text-gray-900 mb-2">Session Timeline</h3>
        <p class="text-sm text-gray-600">
          Track the progress and key events of this AI session
        </p>
      </div>

      {/* Timeline content */}
      <div 
        class="space-y-6"
        role="list"
        aria-label="Session timeline events"
      >
        <For each={timelineEvents()}>
          {(event, index) => (
            <div role="listitem">
              <TimelineEventItem 
                event={event}
                isLast={index() === timelineEvents().length - 1 && !isSessionRunning()}
              />
            </div>
          )}
        </For>

        {/* Live progress indicator for running sessions */}
        <Show when={isSessionRunning()}>
          <div role="listitem">
            <LiveProgressIndicator session={session()} />
          </div>
        </Show>
      </div>

      {/* Timeline summary */}
      <div class="mt-6 pt-4 border-t border-gray-200">
        <div class="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span class="text-gray-500">Total Events:</span>
            <span class="ml-2 font-medium text-gray-900">{timelineEvents().length}</span>
          </div>
          <div>
            <span class="text-gray-500">Session Status:</span>
            <span class="ml-2 font-medium text-gray-900 capitalize">{session().status}</span>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SessionTimeline;
