import { Component, Show } from 'solid-js';
import { A } from '@solidjs/router';
import { AiSessionStatus, ExtendedAiSession, Project } from '../types';

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
const ToolIcon: Component<{
  toolName: string | null | undefined;
  model?: string | null;
  className?: string;
}> = props => {
  const getToolDisplayName = (tool: string | null | undefined, model?: string | null) => {
    if (!tool) return 'Unknown';
    const toolLower = tool.toLowerCase();

    // Check if tool_name contains "LLM Agent (model-name)" format
    const llmAgentMatch = tool.match(/^LLM Agent \((.+)\)$/i);
    if (llmAgentMatch) {
      return llmAgentMatch[1]; // Extract the model name
    }

    if (toolLower.includes('llm-agent')) {
      return model || 'Model';
    }
    return tool;
  };

  return (
    <WorkWidget
      type='model'
      value={getToolDisplayName(props.toolName, props.model)}
      className={props.className}
    />
  );
};

// Unified widget component for Status, Model, and Project badges
const WorkWidget: Component<{
  type: 'status' | 'model' | 'project';
  value: string;
  status?: AiSessionStatus;
  project?: Project | null;
  projectId?: string;
  className?: string;
}> = props => {
  const getWidgetColor = () => {
    switch (props.type) {
      case 'status':
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
      case 'model':
        return 'bg-purple-100 text-purple-800 border-purple-200';
      case 'project':
        return 'bg-indigo-100 text-indigo-800 border-indigo-200';
      default:
        return 'bg-gray-100 text-gray-800 border-gray-200';
    }
  };

  const getDisplayText = () => {
    switch (props.type) {
      case 'status':
        return props.value.charAt(0).toUpperCase() + props.value.slice(1);
      case 'model':
        return props.value;
      case 'project':
        if (props.project) {
          return props.project.name;
        }
        return props.projectId ? `Project ${props.projectId}` : 'No Project';
      default:
        return props.value;
    }
  };

  return (
    <span
      class={`px-2 py-1 text-xs font-medium rounded-full border inline-flex items-center ${props.className || ''}`}
      classList={{
        [getWidgetColor()]: true,
        'animate-pulse': props.type === 'status' && props.status === 'running',
      }}
      title={`${props.type.charAt(0).toUpperCase() + props.type.slice(1)}: ${getDisplayText()}`}
      aria-label={`${props.type.charAt(0).toUpperCase() + props.type.slice(1)}: ${getDisplayText()}`}
    >
      <span class='truncate max-w-24'>{getDisplayText()}</span>
    </span>
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
      aria-label={`Work status: ${props.status}`}
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
  const projectName =
    props.project?.name || (props.projectId ? `Project ${props.projectId}` : 'No Project');

  return (
    <WorkWidget
      type='project'
      value={projectName}
      project={props.project}
      projectId={props.projectId}
    />
  );
};

// Props interface for SessionRow
interface SessionRowProps {
  session: ExtendedAiSession;
  project?: Project | null;
  showPrompt?: boolean;
  class?: string;
}

// Main SessionRow component
const SessionRow: Component<SessionRowProps> = props => {
  const session = () => props.session;

  return (
    <li class={props.class}>
      <A
        href={`/work/${session().id}`}
        class='block hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-inset transition-colors duration-200'
        role='article'
        aria-label={`View details for ${session().tool_name} work`}
      >
        <div class='px-4 py-4 sm:px-6'>
          {/* Header row with status and tool */}
          <div class='flex items-center justify-between mb-2'>
            <div class='flex items-center space-x-3'>
              <StatusBadge status={session().status as AiSessionStatus} size='sm' showIcon={true} />
              <ToolIcon toolName={session().tool_name} model={session().model} />
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
              projectId={session().project_id?.toString() ?? undefined}
            />
            <div class='text-xs text-gray-500'>
              <span title='Work duration'>
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
              <span>Work is actively running</span>
            </div>
          </Show>
        </div>
      </A>
    </li>
  );
};

export default SessionRow;
export { StatusBadge, ToolIcon, ProjectBadge, WorkWidget };
