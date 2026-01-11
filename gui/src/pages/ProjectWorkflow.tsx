import { createSignal, type Component, For } from 'solid-js';
import WorkflowStepItem from '../components/WorkflowStepItem';
import type { Workflow, WorkflowStep } from '../../api-types/types';

interface WorkflowBranch {
  parentWorkflowId: number | null;
  parentWorkflowName: string | null;
  branchCondition: string | null;
}

interface SiblingBranch {
  id: number;
  name: string;
  condition: string;
}

const ProjectWorkflow: Component = () => {
  const [branchInfo, setBranchInfo] = createSignal<WorkflowBranch>({
    parentWorkflowId: 1,
    parentWorkflowName: 'Incoming Message Handler',
    branchCondition: 'Order Intent Detected',
  });

  const [siblingBranches, setSiblingBranches] = createSignal<SiblingBranch[]>([
    { id: 2, name: 'General Inquiry Handler', condition: 'Question Detected' },
    { id: 3, name: 'Greeting Response', condition: 'Greeting Detected' },
    { id: 4, name: 'Spam Filter', condition: 'Spam/Invalid Message' },
  ]);

  const [steps, setSteps] = createSignal<WorkflowStep[]>([
    {
      id: 1,
      workflow_id: 1,
      step_number: 1,
      description:
        'Extract order details from the validated order message. Parse product names, quantities, and delivery requirements.',
      created_at: BigInt(Date.now()),
    },
    {
      id: 2,
      workflow_id: 1,
      step_number: 2,
      description:
        'Team member receives notification of new order. Reviews extracted information for accuracy and completeness.',
      created_at: BigInt(Date.now()),
    },
    {
      id: 3,
      workflow_id: 1,
      step_number: 3,
      description:
        'Inventory checking tool queries current stock levels for requested products. System highlights any items with insufficient stock.',
      created_at: BigInt(Date.now()),
    },
    {
      id: 4,
      workflow_id: 1,
      step_number: 4,
      description:
        'Team member reviews inventory availability. If items are out of stock, they contact customer via WhatsApp to suggest alternatives or adjusted quantities.',
      created_at: BigInt(Date.now()),
    },
    {
      id: 5,
      workflow_id: 1,
      step_number: 5,
      description:
        'Once inventory is confirmed, team member creates order in internal ERP system. Order automation agent fills in customer details, product codes, and pricing.',
      created_at: BigInt(Date.now()),
    },
    {
      id: 6,
      workflow_id: 1,
      step_number: 6,
      description:
        'Order confirmation is automatically generated and sent to customer via their preferred channel (email or WhatsApp). Includes order number, items, total, and expected delivery date.',
      created_at: BigInt(Date.now()),
    },
    {
      id: 7,
      workflow_id: 1,
      step_number: 7,
      description:
        'Warehouse receives order notification. Picking agent generates optimized pick list and updates order status to "In Progress".',
      created_at: BigInt(Date.now()),
    },
  ]);
  const [nextId, setNextId] = createSignal(9);

  const addStep = () => {
    const currentSteps = steps();
    const maxStepNumber = Math.max(
      ...currentSteps.map((s) => s.step_number),
      0
    );
    setSteps([
      ...currentSteps,
      {
        id: nextId(),
        workflow_id: 1,
        step_number: maxStepNumber + 1,
        description: '',
        created_at: BigInt(Date.now()),
      },
    ]);
    setNextId(nextId() + 1);
  };

  const removeStep = (id: number) => {
    if (steps().length > 1) {
      setSteps(steps().filter((step) => step.id !== id));
    }
  };

  const updateStep = (id: number, description: string) => {
    setSteps(
      steps().map((step) => (step.id === id ? { ...step, description } : step))
    );
  };

  const saveWorkflow = () => {
    const workflowData = {
      workflow: steps().map((step) => ({
        id: step.id,
        step_number: step.step_number,
        description: step.description,
      })),
    };
    localStorage.setItem('workflow_steps', JSON.stringify(workflowData));
    console.log('Workflow saved:', JSON.stringify(workflowData, null, 2));
  };

  return (
    <div class="card bg-base-100 shadow-xl max-w-3xl">
      <div class="card-body">
        <h1 class="text-3xl font-bold">Workflow</h1>
        <div class="flex justify-between items-center mb-6">
          <h2 class="text-xl font-bold">Order Handler</h2>
          <button
            type="button"
            class="btn btn-primary btn-sm"
            onClick={addStep}
          >
            Add Step
          </button>
        </div>

        {branchInfo().parentWorkflowId && (
          <div class="mb-6">
            <div class="alert alert-info">
              <div class="flex items-start gap-4 w-full">
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  fill="none"
                  viewBox="0 0 24 24"
                  class="stroke-current shrink-0 w-6 h-6"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                  />
                </svg>
                <div class="flex-1">
                  <div class="flex items-center gap-2 mb-2">
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      class="h-5 w-5"
                      viewBox="0 0 20 20"
                      fill="currentColor"
                    >
                      <path
                        fill-rule="evenodd"
                        d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
                        clip-rule="evenodd"
                      />
                    </svg>
                    <span class="font-medium">Branched from:</span>
                    <a href="#" class="link link-hover font-semibold">
                      {branchInfo().parentWorkflowName}
                    </a>
                  </div>
                  <div class="flex items-center gap-2">
                    <span class="text-sm">Triggered when:</span>
                    <span class="badge badge-primary badge-sm">
                      {branchInfo().branchCondition}
                    </span>
                  </div>
                </div>
              </div>
            </div>

            <details class="collapse collapse-arrow bg-base-200 mt-2">
              <summary class="collapse-title text-sm font-medium">
                Other branches from "{branchInfo().parentWorkflowName}"
              </summary>
              <div class="collapse-content">
                <div class="space-y-2 pt-2">
                  <For each={siblingBranches()}>
                    {(branch) => (
                      <div class="flex items-center justify-between p-2 bg-base-100 rounded-lg">
                        <div class="flex items-center gap-2">
                          <svg
                            xmlns="http://www.w3.org/2000/svg"
                            class="h-4 w-4 text-base-content/50"
                            viewBox="0 0 20 20"
                            fill="currentColor"
                          >
                            <path
                              fill-rule="evenodd"
                              d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
                              clip-rule="evenodd"
                            />
                          </svg>
                          <a href="#" class="link link-hover text-sm">
                            {branch.name}
                          </a>
                        </div>
                        <span class="badge badge-outline badge-xs">
                          {branch.condition}
                        </span>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            </details>
          </div>
        )}

        <div>
          <h2 class="card-title mb-2">Workflow Steps</h2>
          <p class="text-sm text-base-content/70 mb-6">
            Define the steps of your workflow. Describe how tools, agents, and
            user interactions work together in your process.
          </p>

          <div class="space-y-4">
            <For each={steps()}>
              {(step) => (
                <WorkflowStepItem
                  stepNumber={step.step_number}
                  description={step.description}
                  canRemove={steps().length > 1}
                  onUpdate={(description) => updateStep(step.id, description)}
                  onRemove={() => removeStep(step.id)}
                />
              )}
            </For>
          </div>

          <div class="card-actions justify-end mt-6">
            <button type="button" class="btn btn-outline">
              Cancel
            </button>
            <button
              type="button"
              class="btn btn-primary"
              onClick={saveWorkflow}
            >
              Save Workflow
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};

export default ProjectWorkflow;
