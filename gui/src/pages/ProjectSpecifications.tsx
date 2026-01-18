import {
  createSignal,
  type Component,
  For,
  onMount,
  onCleanup,
  Show,
  createMemo,
} from 'solid-js';
import { useSearchParams } from '@solidjs/router';
import type {
  AgentExecutionRequest,
  AgentExecutionResponse,
  SessionResponse,
  AskUserRequest,
  UserQuestion,
} from '../../api-types/types';

interface ClarificationRound {
  questions: UserQuestion[];
  answers: Map<string, string>;
}

const ProjectRequirements: Component = () => {
  const [searchParams] = useSearchParams();

  // Session tracking
  const [sessionId, setSessionId] = createSignal<bigint | null>(null);
  const [isPolling, setIsPolling] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  // Agent result
  const [questions, setQuestions] = createSignal<UserQuestion[]>([]);
  const [isCompleted, setIsCompleted] = createSignal(false);

  // User input
  const [answers, setAnswers] = createSignal<Map<string, string>>(new Map());

  // Conversation history
  const [originalPrompt, setOriginalPrompt] = createSignal('');
  const [clarificationHistory, setClarificationHistory] = createSignal<
    ClarificationRound[]
  >([]);

  let pollInterval: number | undefined;

  const allQuestionsAnswered = createMemo(() => {
    const currentQuestions = questions();
    const currentAnswers = answers();
    return currentQuestions.every(
      (q) => currentAnswers.get(q.id)?.trim() !== ''
    );
  });

  const buildPromptWithHistory = (): string => {
    const prompt = originalPrompt();
    const history = clarificationHistory();

    if (history.length === 0) {
      return prompt;
    }

    let fullPrompt = prompt + '\n\nPrevious clarifications:\n';

    history.forEach((round, index) => {
      fullPrompt += `\nRound ${index + 1}:\n`;
      round.questions.forEach((question) => {
        const answer = round.answers.get(question.id) || '';
        fullPrompt += `Question: ${question.question}\nAnswer: ${answer}\n\n`;
      });
    });

    return fullPrompt;
  };

  const triggerClarificationAgent = async (prompt: string) => {
    try {
      setError(null);
      setIsPolling(true);

      const requestBody: AgentExecutionRequest = {
        user_prompt: prompt,
        config: {
          type: 'user-clarification',
        },
      };

      const response = await fetch(
        'http://127.0.0.1:8080/agents/user-clarification/execute',
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(requestBody),
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to start agent: ${response.statusText}`);
      }

      const result: AgentExecutionResponse = await response.json();
      setSessionId(result.session_id);

      // Start polling
      pollInterval = setInterval(
        () => pollSession(result.session_id),
        2000
      ) as unknown as number;
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'Failed to start clarification'
      );
      setIsPolling(false);
    }
  };

  const pollSession = async (sessionId: bigint) => {
    try {
      const response = await fetch(
        `http://127.0.0.1:8080/agents/sessions/${sessionId}`
      );
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data: SessionResponse = await response.json();

      // Handle waiting_for_user_input status - fetch questions
      if (data.status === 'waiting_for_user_input') {
        setIsPolling(false);
        if (pollInterval) {
          clearInterval(pollInterval);
          pollInterval = undefined;
        }

        // Fetch questions from new endpoint
        try {
          const questionsResponse = await fetch(
            `http://127.0.0.1:8080/agents/sessions/${sessionId}/questions`
          );
          if (!questionsResponse.ok) {
            throw new Error(
              `Failed to fetch questions: ${questionsResponse.statusText}`
            );
          }
          const questionsData = await questionsResponse.json();
          setQuestions(questionsData.questions);

          // Reset answers for new questions
          setAnswers(new Map());
        } catch (questionsError) {
          setError('Failed to fetch questions');
          console.error('Questions fetch error:', questionsError);
        }
        return;
      }

      // Stop polling if session is completed or failed
      if (data.status === 'completed' || data.status === 'failed') {
        setIsPolling(false);
        if (pollInterval) {
          clearInterval(pollInterval);
          pollInterval = undefined;
        }

        if (data.status === 'completed') {
          setIsCompleted(true);
          // No more questions - session completed
          setQuestions([]);
        } else if (data.status === 'failed') {
          setError('Agent execution failed');
        }
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to fetch session');
      setIsPolling(false);
      if (pollInterval) {
        clearInterval(pollInterval);
        pollInterval = undefined;
      }
    }
  };

  const handleUpdateSpecifications = async () => {
    if (!allQuestionsAnswered()) {
      setError('Please answer all questions before continuing');
      return;
    }

    const currentSessionId = sessionId();
    if (!currentSessionId) {
      setError('No active session');
      return;
    }

    try {
      setError(null);
      setIsPolling(true);

      // Save current round to history
      const currentRound: ClarificationRound = {
        questions: questions(),
        answers: new Map(answers()),
      };
      setClarificationHistory([...clarificationHistory(), currentRound]);

      // Convert Map to plain object for JSON
      const answersObj: Record<string, string> = {};
      answers().forEach((value, key) => {
        answersObj[key] = value;
      });

      // Submit answers to API
      const response = await fetch(
        `http://127.0.0.1:8080/agents/sessions/${currentSessionId}/answers`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({ answers: answersObj }),
        }
      );

      if (!response.ok) {
        throw new Error(`Failed to submit answers: ${response.statusText}`);
      }

      // Clear current questions and start polling again
      setQuestions([]);

      // Start polling for next round of questions or completion
      pollInterval = setInterval(
        () => pollSession(currentSessionId),
        2000
      ) as unknown as number;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to submit answers');
      setIsPolling(false);
    }
  };

  onMount(() => {
    const prompt = searchParams.prompt;
    if (!prompt) {
      setError('No prompt provided. Please start from the home page.');
      return;
    }

    setOriginalPrompt(prompt);
    triggerClarificationAgent(prompt);
  });

  onCleanup(() => {
    if (pollInterval) {
      clearInterval(pollInterval);
    }
  });

  return (
    <div class="card bg-base-100 shadow-xl max-w-3xl">
      <div class="card-body">
        <h1 class="text-3xl font-bold mb-4">Project Requirements</h1>

        <div class="mb-6">
          <h2 class="text-lg font-semibold mb-2">Original Prompt:</h2>
          <div class="p-4 bg-base-200 rounded-lg">
            <p class="text-base-content/80">{originalPrompt()}</p>
          </div>
        </div>

        <Show when={error()}>
          <div class="alert alert-error mb-6">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              class="stroke-current shrink-0 h-6 w-6"
              fill="none"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <span>{error()}</span>
          </div>
        </Show>

        <Show when={isPolling()}>
          <div class="flex flex-col items-center justify-center py-12">
            <span class="loading loading-spinner loading-lg mb-4"></span>
            <p class="text-base-content/70">
              Analyzing your requirements and preparing questions...
            </p>
          </div>
        </Show>

        <Show when={questions().length > 0 && !isPolling()}>
          <div class="mb-8">
            <h2 class="text-2xl font-bold mb-4">Clarification Questions</h2>
            <p class="text-base-content/70 mb-6">
              Please answer the following questions to help us better understand
              your requirements.
            </p>

            <div class="space-y-6">
              <For each={questions()}>
                {(question) => (
                  <fieldset class="fieldset">
                    <legend class="fieldset-legend">{question.question}</legend>
                    {question.description && (
                      <p class="label">{question.description}</p>
                    )}
                    <textarea
                      id={question.id}
                      class="textarea w-full h-24"
                      placeholder="Type your answer here..."
                      value={answers().get(question.id) || ''}
                      onInput={(e) => {
                        const newAnswers = new Map(answers());
                        newAnswers.set(question.id, e.currentTarget.value);
                        setAnswers(newAnswers);
                      }}
                    />
                  </fieldset>
                )}
              </For>
            </div>

            <div class="mt-8">
              <button
                class="btn btn-primary btn-lg"
                disabled={isPolling() || !allQuestionsAnswered()}
                onClick={handleUpdateSpecifications}
              >
                Update Requirements
              </button>
            </div>
          </div>
        </Show>

        <Show when={isCompleted() && questions().length === 0 && !isPolling()}>
          <div class="alert alert-success">
            <svg
              xmlns="http://www.w3.org/2000/svg"
              class="stroke-current shrink-0 h-6 w-6"
              fill="none"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="2"
                d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <span>
              All clarifications complete! Your requirements are ready.
            </span>
          </div>
        </Show>

        <Show when={clarificationHistory().length > 0}>
          <div class="mt-8">
            <details class="collapse collapse-arrow bg-base-200">
              <summary class="collapse-title text-lg font-medium">
                Previous Clarifications ({clarificationHistory().length} rounds)
              </summary>
              <div class="collapse-content">
                <div class="space-y-6 pt-4">
                  <For each={clarificationHistory()}>
                    {(round, index) => (
                      <div class="card bg-base-100 shadow-sm">
                        <div class="card-body">
                          <h3 class="card-title text-base">
                            Round {index() + 1}
                          </h3>
                          <div class="space-y-4">
                            <For each={round.questions}>
                              {(q) => (
                                <div class="border-l-2 border-primary pl-4">
                                  <p class="font-semibold mb-1">
                                    Q: {q.question}
                                  </p>
                                  <p class="text-base-content/70">
                                    A: {round.answers.get(q.id)}
                                  </p>
                                </div>
                              )}
                            </For>
                          </div>
                        </div>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            </details>
          </div>
        </Show>
      </div>
    </div>
  );
};

export default ProjectRequirements;
