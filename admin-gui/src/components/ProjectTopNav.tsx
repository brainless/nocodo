import type { JSX } from 'solid-js';

type ProjectTopNavProps = {
  title: string;
  actions?: JSX.Element;
};

export default function ProjectTopNav(props: ProjectTopNavProps) {
  return (
    <div class="project-topnav">
      <span class="project-topnav-title">{props.title}</span>
      <div class="project-topnav-actions">
        <label for="chat-drawer" class="btn btn-success btn-sm">Dev Team</label>
        {props.actions}
      </div>
    </div>
  );
}
