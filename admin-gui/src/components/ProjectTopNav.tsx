import type { JSX } from 'solid-js';
import { Show } from 'solid-js';
import { A, useParams, useLocation } from '@solidjs/router';

type ProjectTopNavProps = {
  title: string;
  actions?: JSX.Element;
  backHref?: string;
};

export default function ProjectTopNav(props: ProjectTopNavProps) {
  const params = useParams<{ projectId: string }>();
  const location = useLocation();

  const isOnChatPage = () =>
    location.pathname.startsWith(`/admin/projects/${params.projectId}/chat`);

  return (
    <div class="project-topnav">
      <div class="flex items-center gap-2 min-w-0">
        <Show when={props.backHref}>
          <A href={props.backHref!} class="btn btn-ghost btn-sm gap-1 flex-shrink-0 md:hidden">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M19 12H5M12 5l-7 7 7 7"/>
            </svg>
            Sessions
          </A>
        </Show>
        <span class={`project-topnav-title truncate ${props.backHref ? 'hidden md:inline' : ''}`}>
          {props.title}
        </span>
      </div>
      <div class="project-topnav-actions">
        <Show when={!isOnChatPage()}>
          <A href={`/projects/${params.projectId}/chat`} class="btn btn-success btn-sm">Chat with nocodo</A>
        </Show>
        <Show when={isOnChatPage()}>
          <A href={`/projects/${params.projectId}/chat`} class="btn btn-sm btn-outline">New Chat</A>
        </Show>
        {props.actions}
      </div>
    </div>
  );
}
