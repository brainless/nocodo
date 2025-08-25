import { Component, Show } from 'solid-js';
import { A } from '@solidjs/router';
import { AiSession, AiSessionStatus, Project } from '../types';

// Utility function to format timestamps
const formatTimestamp = (timestamp: number): string => {
  const date = new Date(timestamp * 1000);
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
const formatDuration = (startedAt: number, endedAt?: number): string => {
  const start = new Date(startedAt * 1000);
  const end = endedAt ? new Date(endedAt * 1000) : new Date();
  const durationMs = end.getTime() - start.getTime();

  const minutes = Math.floor(durationMs / 60000);
  const seconds = Math.floor((durationMs % 60000) / 1000);

  if (minutes > 0) {
    return `${minutes}m ${seconds}s`;
  }
  return `${seconds}s`;
};

// Tool icon component with support for different AI tools
const ToolIcon: Component<{ toolName: string; className?: string }> = props => {
  const getToolIcon = (tool: string) => {
    const toolLower = tool.toLowerCase();
    if (toolLower.includes('claude')) {
      return '🤖'; // Claude icon
    }
    if (toolLower.includes('gpt') || toolLower.includes('openai')) {
      return '🧠'; // GPT icon
    }
    if (toolLower.includes('gemini')) {
      return '💎'; // Gemini icon
    }
    if (toolLower.includes('qwen')) {
      return '🔷'; // Qwen icon
    }
    return '⚡'; // Default AI icon
  };

  const getToolColor = (tool: string) => {
    const toolLower = tool.toLowerCase();
    if (toolLower.includes('claude')) {
      return 'bg-orange-100 text-orange-800 border-orange-200';
    }
    if (toolLower.includes('gpt') || toolLower.includes('openai')) {
      return 'bg-green-100 text-green-800 border-green-200';
    }
    if (toolLower.includes('gemini')) {
      return 'bg-purple-100 text-purple-800 border-purple-200';
    }
    if (toolLower.includes('qwen')) {
      return 'bg-blue-100 text-blue-800 border-blue-200';
    }
    return 'bg-gray-100 text-gray-800 border-gray-200';
  };

  return (
    <div
      class={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium border ${getToolColor(props.toolName)} ${props.className || ''}`}
      title={`AI Tool: ${props.toolName}`}
      aria-label={`AI Tool: ${props.toolName}`}
    >
      <span class='mr-1' aria-hidden='true'>
        {getToolIcon(props.toolName)}
      </span>
      <span class='truncate max-w-24'>{props.toolName}</span>
    </div>
  );
};

// Enhanced status badge component with animations and icons
const StatusBadge: Component<{
  status: AiSessionStatus;
  size?: 'sm' | 'md';
  showIcon?: boolean;
}> = props => {
  const getStatusColor = () => {
    switch (props.status) {
      case 'completed':
        return 'bg-green-100 text-green-800 border-green-200';
      case 'running':
      case 'started':
        return 'bg-blue-100 text-blue-800 border-blue-200';
      case 'failed':
        return 'bg-red-100 text-red-800 border-red-200';
      case 'cancelled':
        return 'bg-gray-100 text-gray-800 border-gray-200';
      case 'pending':
        return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      default:
        return 'bg-gray-100 text-gray-800 border-gray-200';
    }
  };

  const getStatusIcon = () => {
    switch (props.status) {
      case 'completed':
        return '✅';
      case 'running':
      case 'started':
        return '⟳';
      case 'failed':
        return '❌';
      case 'cancelled':
        return '⚪';
      case 'pending':
        return '⏳';
      default:
        return '❓';
    }
  };

  const sizeClasses = props.size === 'md' ? 'px-3 py-2 text-sm' : 'px-2 py-1 text-xs';

  return (
    <span
      class={`${sizeClasses} font-medium rounded-full border ${getStatusColor()} inline-flex items-center gap-1`}
      title={`Status: ${props.status}`}
      aria-label={`Session status: ${props.status}`}
    >
      <Show when={props.showIcon !== false}>
        <span class={props.status === 'running' ? 'animate-spin' : ''} aria-hidden='true'>
          {getStatusIcon()}
        </span>
      </Show>
      <span class='capitalize'>{props.status}</span>
    </span>
  );
};

// Project badge component
const ProjectBadge: Component<{
  project: Project | null;
  projectId?: string;
}> = props => {
  if (!props.project && !props.projectId) {
    return (
      <span class='inline-flex items-center px-2 py-1 rounded-md text-xs font-medium bg-gray-100 text-gray-600 border border-gray-200'>
        <span class='mr-1' aria-hidden='true'>
          📁
        </span>
        No Project
      </span>
    );
  }

  const projectName = props.project?.name || `Project ${props.projectId}`;

  return (
    <span class='inline-flex items-center px-2 py-1 rounded-md text-xs font-medium bg-indigo-100 text-indigo-800 border border-indigo-200'>
      <span class='mr-1' aria-hidden='true'>
        📂
      </span>
      <span class='truncate max-w-32'>{projectName}</span>
    </span>
  );
};

// Props interface for SessionRow
interface SessionRowProps {
  session: AiSession;
  project?: Project | null;
  showPrompt?: boolean;
  className?: string;
}

// Main SessionRow component
const SessionRow: Component<SessionRowProps> = props => {
  const session = () => props.session;

  return (
    <li class={props.className}>
      <A
        href={`/ai/sessions/${session().id}`}
        class='block hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-inset transition-colors duration-200'
        role='article'
        aria-label={`View details for ${session().tool_name} session`}
      >
        <div class='px-4 py-4 sm:px-6'>
          {/* Header row with status and tool */}
          <div class='flex items-center justify-between mb-2'>
            <div class='flex items-center space-x-3'>
              <StatusBadge status={session().status as AiSessionStatus} size='sm' showIcon={true} />
              <ToolIcon toolName={session().tool_name} />
            </div>
            <div class='flex items-center space-x-2 text-sm text-gray-500'>
              <time
                dateTime={new Date(session().started_at * 1000).toISOString()}
                title={new Date(session().started_at * 1000).toLocaleString()}
              >
                {formatTimestamp(session().started_at)}
              </time>
            </div>
          </div>

          {/* Project and duration row */}
          <div class='flex items-center justify-between mb-2'>
            <ProjectBadge
              project={props.project ?? null}
              projectId={session().project_id ?? undefined}
            />
            <div class='text-xs text-gray-500'>
              <span title='Session duration'>
                Duration: {formatDuration(session().started_at, session().ended_at ?? undefined)}
                <Show when={!session().ended_at}>
                  <span class='text-blue-600'> (ongoing)</span>
                </Show>
              </span>
            </div>
          </div>

          {/* Prompt preview */}
          <Show when={props.showPrompt !== false && session().prompt}>
            <div class='mt-2'>
              <p
                class='text-sm text-gray-700 line-clamp-2 leading-relaxed'
                title={session().prompt}
              >
                {session().prompt}
              </p>
            </div>
          </Show>

          {/* Visual indicator for running sessions */}
          <Show when={session().status === 'running' || session().status === 'started'}>
            <div class='mt-2 flex items-center text-xs text-blue-600'>
              <div
                class='w-1.5 h-1.5 bg-blue-400 rounded-full animate-pulse mr-2'
                aria-hidden='true'
              ></div>
              <span>Session is actively running</span>
            </div>
          </Show>
        </div>
      </A>
    </li>
  );
};

export default SessionRow;
export { StatusBadge, ToolIcon, ProjectBadge };
