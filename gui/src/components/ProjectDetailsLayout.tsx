import { type ParentComponent } from 'solid-js';
import { A, useParams } from '@solidjs/router';

const ProjectDetailsLayout: ParentComponent = (props) => {
  const params = useParams();

  return (
    <div class="flex min-h-screen">
      {/* Left Sidebar */}
      <aside class="w-64 bg-base-200 p-4">
        <div class="mb-6">
          <h2 class="text-xl font-bold mb-2">Project</h2>
          <p class="text-sm text-base-content/70">ID: {params.projectId}</p>
        </div>

        <ul class="menu bg-base-100 rounded-box w-full">
          <li>
            <A href={`/projects/${params.projectId}/specifications`}>
              Specifications
            </A>
          </li>
          <li>
            <A href={`/projects/${params.projectId}/workflow`}>Workflow</A>
          </li>
          <li>
            <A href={`/projects/${params.projectId}/process`}>Process</A>
          </li>
          <li>
            <A href={`/projects/${params.projectId}/data-sources`}>
              Data sources
            </A>
          </li>
        </ul>
      </aside>

      {/* Main Content */}
      <main class="flex-1 p-8">{props.children}</main>
    </div>
  );
};

export default ProjectDetailsLayout;
