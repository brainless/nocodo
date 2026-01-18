import { type Component } from 'solid-js';

interface WorkflowStepItemProps {
  stepNumber: number;
  description: string;
  canRemove: boolean;
  onUpdate: (description: string) => void;
  onRemove: () => void;
}

const WorkflowStepItem: Component<WorkflowStepItemProps> = (props) => {
  return (
    <div class="form-control">
      <div class="mb-2">
        <span class="text-sm font-medium">Step {props.stepNumber}</span>
      </div>
      <textarea
        class="textarea textarea-bordered h-24 w-full rounded-md"
        placeholder="Describe this workflow step (e.g., 'User submits a request, agent processes it using tool X, then notifies user via email')"
        value={props.description}
        onInput={(e) => props.onUpdate(e.currentTarget.value)}
      />
      <div class="mt-2 flex justify-between items-center">
        <div>
          <button type="button" class="btn btn-outline btn-xs">
            Agent Settings
          </button>
        </div>
        <div>
          {props.canRemove && (
            <button
              type="button"
              class="btn btn-ghost btn-xs"
              onClick={props.onRemove}
            >
              Remove
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

export default WorkflowStepItem;
