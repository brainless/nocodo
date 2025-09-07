import { Component, Show } from 'solid-js';
import { A } from '@solidjs/router';
import { AiSessionStatus, ExtendedAiSession, Project } from '../types';
import { ProjectBadge, StatusBadge, ToolIcon } from './SessionRow';

interface AiSessionCardProps {
  session: ExtendedAiSession;
  project?: Project | null;
  showPrompt?: boolean;
}

// Utility function to format timestamps
const formatTimestamp = (timestamp: number | null | undefined): string => {
  if (!timestamp || typeof timestamp !== 'number' || timestamp <= 0) {
    return 'Unknown';
  }

  const date = new Date(timestamp * 1000);
  if (isNaN(date.getTime())) {
    return 'Unknown';
  }

  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMinutes = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMinutes / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffDays > 0) {
    return `${diffDays}d ago`;
  }
  if (diffHours > 0) {
    return `${diffHours}h ago`;
  }
  if (diffMinutes > 0) {
    return `${diffMinutes}m ago`;
  }
  return 'Just now';
};

// Utility function to format duration
const formatDuration = (startedAt: number | null | undefined, endedAt?: number | null): string => {
  if (!startedAt || typeof startedAt !== 'number' || startedAt <= 0) {
    return 'Unknown';
  }

  const start = new Date(startedAt * 1000);
  if (isNaN(start.getTime())) {
    return 'Unknown';
  }

  const end = endedAt && endedAt > 0 ? new Date(endedAt * 1000) : new Date();
  if (isNaN(end.getTime())) {
    return 'Unknown';
  }

  const durationMs = end.getTime() - start.getTime();

  const minutes = Math.floor(durationMs / 60000);
  const seconds = Math.floor((durationMs % 60000) / 1000);

  if (minutes > 0) {
    return `${minutes}m ${seconds}s`;
  }
  return `${seconds}s`;
};

const AiSessionCard: Component<AiSessionCardProps> = props => {
  return (
    <div class='bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow'>
      <A href={`/work/${props.session.id}`} class='block'>
        {/* Header row with status and tool */}
        <div class='flex items-center justify-between mb-4'>
          <div class='flex items-center space-x-3'>
            <StatusBadge
              status={props.session.status as AiSessionStatus}
              size='sm'
              showIcon={true}
            />
            <ToolIcon toolName={props.session.tool_name} />
          </div>
          <div class='text-sm text-gray-500'>
            <time
              dateTime={
                props.session.started_at && props.session.started_at > 0
                  ? new Date(props.session.started_at * 1000).toISOString()
                  : ''
              }
              title={
                props.session.started_at && props.session.started_at > 0
                  ? new Date(props.session.started_at * 1000).toLocaleString()
                  : 'Unknown timestamp'
              }
            >
              {formatTimestamp(props.session.started_at)}
            </time>
          </div>
        </div>

        {/* Project and duration row */}
        <div class='flex items-center justify-between mb-4'>
          <ProjectBadge
            project={props.project ?? null}
            projectId={props.session.project_id ?? undefined}
          />
          <div class='text-xs text-gray-500'>
            <span title='Session duration'>
              Duration:{' '}
              {formatDuration(props.session.started_at, props.session.ended_at ?? undefined)}
              <Show when={!props.session.ended_at}>
                <span class='text-blue-600'> (ongoing)</span>
              </Show>
            </span>
          </div>
        </div>

        {/* Prompt preview */}
        <Show when={props.showPrompt !== false && props.session.prompt}>
          <div class='mb-4'>
            <p
              class='text-sm text-gray-700 line-clamp-2 leading-relaxed'
              title={props.session.prompt}
            >
              {props.session.prompt}
            </p>
          </div>
        </Show>

        {/* Visual indicator for running sessions */}
        <Show when={props.session.status === 'running' || props.session.status === 'started'}>
          <div class='flex items-center text-xs text-blue-600'>
            <div
              class='w-1.5 h-1.5 bg-blue-400 rounded-full animate-pulse mr-2'
              aria-hidden='true'
            ></div>
            <span>Work is actively running</span>
          </div>
        </Show>
      </A>
    </div>
  );
};

export default AiSessionCard;
