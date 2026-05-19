import type { JSX } from 'solid-js';
import { A, useParams } from '@solidjs/router';

type ProjectTopNavProps = {
  title: string;
  actions?: JSX.Element;
};

export default function ProjectTopNav(props: ProjectTopNavProps) {
  const params = useParams<{ projectId: string }>();

  return (
    <div class="project-topnav">
      <span class="project-topnav-title">{props.title}</span>
      <div class="project-topnav-actions">
        <A href={`/projects/${params.projectId}/chat`} class="btn btn-success btn-sm">Chat with nocodo</A>
        {props.actions}
      </div>
    </div>
  );
}
