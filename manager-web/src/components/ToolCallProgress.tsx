import { Component } from 'solid-js';
import { ToolCallState } from '../stores/sessionsStore';

const ToolCallProgress: Component<{ toolCall: ToolCallState }> = ({ toolCall }) => {
  const getStatusColor = () => {
    switch (toolCall.status) {
      case 'pending':
        return 'border-gray-300 bg-gray-50';
      case 'executing':
        return 'border-yellow-400 bg-yellow-50';
      case 'completed':
        return 'border-green-400 bg-green-50';
      case 'failed':
        return 'border-red-400 bg-red-50';
      default:
        return 'border-gray-300 bg-gray-50';
    }
  };

  const getStatusIcon = () => {
    switch (toolCall.status) {
      case 'pending':
        return '⏳';
      case 'executing':
        return '⚙️';
      case 'completed':
        return '✅';
      case 'failed':
        return '❌';
      default:
        return '❓';
    }
  };

  const getStatusText = () => {
    switch (toolCall.status) {
      case 'pending':
        return 'Pending';
      case 'executing':
        return 'Executing';
      case 'completed':
        return 'Completed';
      case 'failed':
        return 'Failed';
      default:
        return 'Unknown';
    }
  };

  return (
    <div class={`border-l-4 p-4 my-2 rounded-r-md ${getStatusColor()}`}>
      <div class='flex items-center'>
        <span class='text-lg mr-2' role='img' aria-label={toolCall.status}>
          {getStatusIcon()}
        </span>
        <div class='flex-1'>
          <div class='flex items-center justify-between'>
            <span class='font-medium text-gray-900'>{toolCall.toolName}</span>
            <span
              class={`text-sm font-medium ${
                toolCall.status === 'executing'
                  ? 'text-yellow-700'
                  : toolCall.status === 'completed'
                    ? 'text-green-700'
                    : toolCall.status === 'failed'
                      ? 'text-red-700'
                      : 'text-gray-600'
              }`}
            >
              {getStatusText()}
            </span>
          </div>
          {toolCall.status === 'executing' && (
            <div class='mt-1'>
              <div class='animate-pulse bg-yellow-200 h-2 rounded' style='width: 60%'></div>
            </div>
          )}
          {toolCall.status === 'completed' && toolCall.result && (
            <details class='mt-2'>
              <summary class='cursor-pointer text-sm text-gray-600 hover:text-gray-800'>
                View Result
              </summary>
              <pre class='mt-1 text-xs bg-gray-100 p-2 rounded overflow-x-auto'>
                {JSON.stringify(toolCall.result, null, 2)}
              </pre>
            </details>
          )}
          {toolCall.status === 'failed' && toolCall.error && (
            <div class='mt-2'>
              <p class='text-sm text-red-700 font-medium'>Error:</p>
              <p class='text-sm text-red-600 mt-1'>{toolCall.error}</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default ToolCallProgress;
