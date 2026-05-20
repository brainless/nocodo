import { A, useParams } from '@solidjs/router';
import type { RouteSectionProps } from '@solidjs/router';
import { For, Show, createEffect, createSignal } from 'solid-js';
import { ProjectProvider } from '../contexts/ProjectContext';
import type { ListTasksResponse } from '../types/api';

const NAV_ITEMS = [
  {
    href: 'chat',
    agentType: '',
    label: 'Chat with nocodo',
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
      </svg>
    ),
  },
  {
    href: 'epics',
    agentType: 'project_manager',
    label: 'Epics/Tasks',
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/>
        <rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>
      </svg>
    ),
  },
  {
    href: 'database',
    agentType: 'db_engineer',
    label: 'Database',
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <ellipse cx="12" cy="5" rx="9" ry="3"/>
        <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>
        <path d="M3 12c0 1.66 4 3 9 3s9-1.34 9-3"/>
      </svg>
    ),
  },
  {
    href: 'backend',
    agentType: 'backend_engineer',
    label: 'Backend',
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="16 18 22 12 16 6"/>
        <polyline points="8 6 2 12 8 18"/>
      </svg>
    ),
  },
  {
    href: 'ui-design',
    agentType: 'ui_designer',
    label: 'UI Design',
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10"/>
        <path d="M12 2a7 7 0 0 1 7 7c0 4.97-7 13-7 13S5 13.97 5 9a7 7 0 0 1 7-7z"/>
        <circle cx="12" cy="9" r="2"/>
      </svg>
    ),
  },
];

function ProjectSidebar() {
  const params = useParams<{ projectId: string }>();
  const base = `/projects/${params.projectId}`;
  const [activeAgents, setActiveAgents] = createSignal(new Set<string>());

  createEffect(() => {
    const pid = params.projectId;
    if (!pid) return;
    fetch(`/api/agents/tasks?project_id=${pid}`)
      .then(r => r.json() as Promise<ListTasksResponse>)
      .then(data => {
        setActiveAgents(new Set((data.tasks ?? []).map(t => t.assigned_to_agent)));
      })
      .catch(() => {});
  });

  return (
    <nav class="project-sidebar">
      {/* Home */}
      <div class="project-sidebar-top">
        <A href="/" class="tooltip tooltip-right project-sidebar-icon" data-tip="Home" end>
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
            <polyline points="9 22 9 12 15 12 15 22"/>
          </svg>
        </A>

        <div class="project-sidebar-divider" />

        <For each={NAV_ITEMS}>
          {(item) => (
            <A
              href={`${base}/${item.href}`}
              class="tooltip tooltip-right project-sidebar-icon"
              activeClass="project-sidebar-icon-active"
              data-tip={item.label}
            >
              <span class="sidebar-icon-wrap">
                {item.icon}
                <Show when={activeAgents().has(item.agentType)}>
                  <span class="sidebar-dot" />
                </Show>
              </span>
            </A>
          )}
        </For>
      </div>

      {/* Bottom actions */}
      <div class="project-sidebar-bottom">
        <A
          href={`${base}/project-notes`}
          class="tooltip tooltip-right project-sidebar-icon"
          activeClass="project-sidebar-icon-active"
          data-tip="Project Notes"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="16" y1="13" x2="8" y2="13"/>
            <line x1="16" y1="17" x2="8" y2="17"/>
            <polyline points="10 9 9 9 8 9"/>
          </svg>
        </A>

        <A
          href={`${base}/stack-notes`}
          class="tooltip tooltip-right project-sidebar-icon"
          activeClass="project-sidebar-icon-active"
          data-tip="Tech Stack"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2 2 7l10 5 10-5-10-5z"/>
            <path d="M2 17l10 5 10-5"/>
            <path d="M2 12l10 5 10-5"/>
          </svg>
        </A>

        <A
          href={`${base}/settings`}
          class="tooltip tooltip-right project-sidebar-icon"
          activeClass="project-sidebar-icon-active"
          data-tip="Settings"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>
            <circle cx="12" cy="12" r="3"/>
          </svg>
        </A>
      </div>
    </nav>
  );
}

export default function ProjectLayout(props: RouteSectionProps) {
  return (
    <ProjectProvider>
      <div class="project-layout">
        <ProjectSidebar />
        <div class="project-content">
          {props.children}
        </div>
      </div>
    </ProjectProvider>
  );
}
