import { Component } from 'solid-js';
import { A } from '@solidjs/router';
import { Project } from '../types';

interface ProjectCardProps {
  project: Project;
  showActions?: boolean; // Show delete button etc.
  onDelete?: (id: string) => void;
}

const ProjectCard: Component<ProjectCardProps> = (props) => {
  // Icon color based on language/status
  const getProjectIconColor = (language?: string | null) => {
    switch (language) {
      case 'Rust':
        return 'from-orange-500 to-red-600';
      case 'JavaScript':
      case 'TypeScript':
        return 'from-blue-500 to-blue-600';
      case 'Python':
        return 'from-yellow-500 to-blue-600';
      case 'Java':
        return 'from-red-500 to-orange-600';
      case 'Go':
        return 'from-blue-400 to-cyan-500';
      default:
        return 'from-gray-500 to-gray-600';
    }
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleDateString();
  };

  const handleDelete = (e: Event) => {
    e.preventDefault();
    e.stopPropagation();
    if (props.onDelete) {
      props.onDelete(props.project.id);
    }
  };

  return (
    <div class='bg-white rounded-lg shadow-sm border border-gray-200 p-6 hover:shadow-md transition-shadow group'>
      <A href={`/projects/${props.project.id}/work`} class='block'>
        <div class='flex items-start justify-between mb-4'>
          {/* Project icon */}
          <div class={`w-10 h-10 bg-gradient-to-br ${getProjectIconColor(props.project.language)} rounded-lg flex items-center justify-center`}>
            <svg class='h-5 w-5 text-white' fill='none' stroke='currentColor' viewBox='0 0 24 24'>
              <path stroke-linecap='round' stroke-linejoin='round' stroke-width={2} d='M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2H5a2 2 0 00-2-2z' />
              <path stroke-linecap='round' stroke-linejoin='round' stroke-width={2} d='M8 5a2 2 0 012-2h4a2 2 0 012 2v6H8V5z' />
            </svg>
          </div>
          
          <div class='flex items-center space-x-2'>
            {/* Language badge */}
            {props.project.language && (
              <span class='px-2 py-1 bg-green-100 text-green-800 text-xs rounded-full font-medium'>
                {props.project.language}
              </span>
            )}
            
            {/* Framework badge */}
            {props.project.framework && (
              <span class='px-2 py-1 bg-blue-100 text-blue-800 text-xs rounded-full font-medium'>
                {props.project.framework}
              </span>
            )}
          </div>
        </div>
        
        {/* Project name */}
        <h3 class='font-semibold text-gray-900 mb-2'>
          {props.project.name}
        </h3>
        
        {/* Project path (as description) */}
        <p class='text-sm text-gray-600 mb-4 truncate' title={props.project.path}>
          {props.project.path}
        </p>
        
        {/* Project metadata */}
        <div class='flex items-center justify-between text-xs text-gray-500'>
          <span>Modified {formatDate(props.project.updated_at)}</span>
          <span class='capitalize'>{props.project.status}</span>
        </div>
      </A>

      {/* Action buttons - only show if showActions is true */}
      {props.showActions && props.onDelete && (
        <div class='mt-4 pt-4 border-t border-gray-100 flex justify-end opacity-0 group-hover:opacity-100 transition-opacity'>
          <button
            onClick={handleDelete}
            class='px-3 py-1.5 text-xs text-red-600 hover:text-red-800 hover:bg-red-50 rounded-md border border-red-200 hover:border-red-300 transition-colors'
            title='Delete project'
          >
            Delete
          </button>
        </div>
      )}
    </div>
  );
};

export default ProjectCard;