import { Component, Show } from 'solid-js';
import { A } from '@solidjs/router';
import { AiSessionStatus, ExtendedAiSession, Project } from '../types';
import { ProjectBadge, ToolIcon, WorkWidget } from './SessionRow';

interface AiSessionCardProps {
  session: ExtendedAiSession;
  project?: Project | null;
  showPrompt?: boolean;
}

const AiSessionCard: Component<AiSessionCardProps> = props => {
  return (
    <div class='bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow'>
      <A href={`/work/${props.session.id}`} class='block'>
        {/* First line: User provided text */}
        <Show when={props.showPrompt !== false && props.session.prompt}>
          <div class='mb-4'>
            <p
              class='text-sm text-gray-900 line-clamp-2 leading-relaxed'
              title={props.session.prompt}
            >
              {props.session.prompt}
            </p>
          </div>
        </Show>

        {/* Second line: Rounded widgets */}
        <div class='flex flex-wrap gap-2'>
          <WorkWidget
            type='status'
            value={props.session.status}
            status={props.session.status as AiSessionStatus}
          />
          <ToolIcon toolName={props.session.tool_name} model={props.session.model} />
          <ProjectBadge
            project={props.project ?? null}
            projectId={props.session.project_id ?? undefined}
          />
        </div>

        {/* Visual indicator for running sessions */}
        <Show when={props.session.status === 'running' || props.session.status === 'started'}>
          <div class='flex items-center text-xs text-blue-600 mt-3'>
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
