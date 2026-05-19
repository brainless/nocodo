import { For, Show, createEffect, createSignal } from 'solid-js';
import { useNavigate, useParams } from '@solidjs/router';
import {
  UserChatProvider,
  useUserChat,
  type StructuredQuestion,
  type StructuredResponse,
} from '../contexts/UserChatContext';

function NamePrompt() {
  const chat = useUserChat();
  const [nameInput, setNameInput] = createSignal('');

  const handleSubmit = () => {
    const name = nameInput().trim();
    if (!name) return;
    chat.setDisplayName(name);
  };

  return (
    <main class="flex items-center justify-center min-h-screen" style="background: #d9e0ee">
      <div class="bg-white p-8 rounded-2xl shadow-xl max-w-md w-full mx-4">
        <h2 class="text-xl font-bold text-[#172033] mb-2">Welcome!</h2>
        <p class="text-sm text-[#5a6f90] mb-6">What should we call you?</p>
        <div class="flex gap-2">
          <input
            class="input input-bordered flex-1"
            placeholder="Your name..."
            value={nameInput()}
            onInput={(e) => setNameInput(e.currentTarget.value)}
            onKeyDown={(e) => { if (e.key === 'Enter') handleSubmit(); }}
          />
          <button
            class="btn btn-primary"
            onClick={handleSubmit}
            disabled={!nameInput().trim()}
          >
            Start chatting
          </button>
        </div>
      </div>
    </main>
  );
}

function StructuredQuestionWidget(props: {
  messageId: number;
  sessionId: number;
  question: StructuredQuestion;
  answered: boolean;
}) {
  const chat = useUserChat();
  const isMultiple = () => props.question.kind.type === 'multiple_choice';
  const options = () => props.question.kind.options;
  const checked = () => chat.structuredSelections().get(props.messageId) ?? [];

  const toggle = (opt: string) => {
    const current = checked();
    let next: string[];
    if (isMultiple()) {
      next = current.includes(opt) ? current.filter(o => o !== opt) : [...current, opt];
    } else {
      next = [opt];
    }
    chat.setStructuredSelection(props.messageId, next);
  };

  const submit = () => {
    if (props.answered) return;
    const selected = checked();
    if (!selected.length) return;
    const response: StructuredResponse = {
      question_message_id: props.messageId,
      selected,
    };
    void chat.sendStructuredResponse(props.sessionId, response);
  };

  return (
    <div class="rounded-2xl px-4 py-3 bg-gray-100 text-[#2f3f5f] rounded-bl-md space-y-2">
      <p class="text-sm font-medium">{props.question.question}</p>
      <Show when={!props.answered}>
        <div class="space-y-1">
          <For each={options()}>
            {(opt) => (
              <label class="flex items-center gap-2 cursor-pointer text-sm">
                <input
                  type={isMultiple() ? 'checkbox' : 'radio'}
                  name={`q-${props.messageId}`}
                  checked={checked().includes(opt)}
                  onChange={() => toggle(opt)}
                  class={`cursor-pointer ${isMultiple() ? 'checkbox checkbox-sm' : 'radio radio-sm'}`}
                />
                {opt}
              </label>
            )}
          </For>
        </div>
        <button
          class="btn btn-sm btn-primary mt-1"
          disabled={!checked().length || chat.loading() || props.answered}
          onClick={submit}
        >
          Submit
        </button>
      </Show>
      <Show when={props.answered}>
        <p class="text-xs text-[#8fa0be] italic">Answered</p>
      </Show>
    </div>
  );
}

function ChatContent(props: { projectId: () => number | undefined }) {
  const params = useParams<{ projectId: string; sessionId?: string }>();
  const navigate = useNavigate();
  const chat = useUserChat();
  const [inputText, setInputText] = createSignal('');

  const pid = () => props.projectId();
  const urlSessionId = () => params.sessionId ? parseInt(params.sessionId) : null;

  const currentSession = () =>
    chat.sessions().find(s => s.id === chat.currentSessionId()) ?? null;

  const questionResponseByQuestionId = () => {
    const byId = new Map<number, StructuredResponse>();
    for (const m of chat.messages()) {
      if (m.content_type !== 'structured_response') continue;
      try {
        const r = JSON.parse(m.content) as StructuredResponse;
        byId.set(r.question_message_id, r);
      } catch {
        // ignore invalid payload
      }
    }
    return byId;
  };

  const unansweredQuestionRows = () => {
    const responses = questionResponseByQuestionId();
    return chat.messages()
      .filter(m => m.content_type === 'structured_question' && m.id > 0 && !responses.has(m.id));
  };

  const answeredResponseForQuestion = (questionMessageId: number) => {
    return questionResponseByQuestionId().get(questionMessageId);
  };

  const unansweredQuestionCount = () => {
    const msgs = chat.messages();
    const questionIds = msgs
      .filter(m => m.content_type === 'structured_question' && m.id > 0)
      .map(m => m.id);
    return questionIds.filter((qid) =>
      !msgs.some(m => {
        if (m.content_type !== 'structured_response') return false;
        try {
          const r = JSON.parse(m.content) as { question_message_id: number };
          return r.question_message_id === qid;
        } catch { return false; }
      })
    ).length;
  };

  const canSend = () =>
    currentSession()?.status !== 'completed' && !chat.loading();

  const handleSend = async () => {
    const msg = inputText().trim();
    if (!msg) return;

    const sid = urlSessionId();
    if (sid === null) {
      const id = pid();
      if (!id) return;
      setInputText('');
      const newSessionId = await chat.startSession(id, msg);
      if (newSessionId) {
        navigate(`/projects/${params.projectId}/chat/${newSessionId}`);
      }
      return;
    }

    if (!canSend()) return;
    setInputText('');
    await chat.sendMessage(sid, msg);
  };

  const handleNewChat = () => {
    navigate(`/projects/${params.projectId}/chat`);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void handleSend();
    }
  };

  return (
    <main class="sheet-app">
      <div class="flex h-full overflow-hidden">
        <aside class="w-72 flex-shrink-0 border-r border-[#c6cfdf] bg-[#f6f8fe] overflow-y-auto flex flex-col">
          <div class="p-3 flex-1">
            <div class="text-xs font-semibold text-[#7a8faf] uppercase tracking-wider mb-3">
              Sessions
            </div>
            <For each={chat.sessions()}>
              {(session) => (
                <div
                  class={`p-3 rounded-lg cursor-pointer mb-1 transition-colors ${
                    session.id === urlSessionId()
                      ? 'bg-[#dde9ff] border border-[#c7d6f0]'
                      : 'hover:bg-[#e7ecf8] border border-transparent'
                  }`}
                  onClick={() => navigate(`/projects/${params.projectId}/chat/${session.id}`)}
                >
                  <div class="flex items-center justify-between mb-1">
                    <span
                      class={`badge badge-sm ${
                        session.status === 'open' ? 'badge-success' : 'badge-ghost'
                      }`}
                    >
                      {session.status}
                    </span>
                    <span class="text-xs text-[#8fa0be]">
                      {new Date(session.created_at).toLocaleDateString()}
                    </span>
                  </div>
                  <div class="text-xs text-[#8fa0be] truncate">
                    Session #{session.id}
                  </div>
                </div>
              )}
            </For>
          </div>
        </aside>

        <div class="flex-1 flex flex-col min-w-0">
          <div class="flex items-center justify-between px-4 py-2 border-b border-[#c6cfdf] bg-[#f6f8fe]">
            <div class="flex items-center gap-2">
              <span class="text-sm font-semibold text-[#2e4a7c]">
                {currentSession() ? `Session #${currentSession()!.id}` : 'User Chat'}
              </span>
              <Show when={chat.loading()}>
                <span class="loading loading-spinner loading-xs opacity-40" />
              </Show>
            </div>
            <button class="btn btn-sm btn-outline" onClick={handleNewChat}>
              New Chat
            </button>
          </div>

          <div class="flex-1 overflow-y-auto p-4 space-y-3">
            <Show when={urlSessionId() !== null && chat.messages().length > 0}>
              <For each={chat.messages()}>
                {(msg) => {
                  const isUser = msg.author_type === 'user';
                  const isPM = msg.agent_type === 'project_manager';
                  const isPO = msg.agent_type === 'product_owner';
                  const isStructuredQuestion = msg.content_type === 'structured_question';
                  const isStructuredResponse = msg.content_type === 'structured_response';

                  // A structured question is "answered" if a later message references it.
                  const isAnswered = () => {
                    if (!isStructuredQuestion || msg.id === 0) return false;
                    return questionResponseByQuestionId().has(msg.id);
                  };

                  // Compact label for structured_response bubbles
                  const responseLabel = () => {
                    if (!isStructuredResponse) return msg.content;
                    try {
                      const r = JSON.parse(msg.content) as { selected: string[] };
                      return `Selected: ${r.selected.join(', ')}`;
                    } catch { return msg.content; }
                  };

                  // Open questions are rendered in the bottom "Open Questions" panel.
                  if (isStructuredQuestion && !isAnswered()) return null;

                  // Structured responses are rendered directly under their answered question.
                  if (isStructuredResponse) {
                    try {
                      const r = JSON.parse(msg.content) as { question_message_id: number };
                      const parentExists = chat.messages().some(
                        m => m.id === r.question_message_id && m.content_type === 'structured_question'
                      );
                      if (parentExists) return null;
                    } catch {
                      // fall through to default text bubble
                    }
                  }

                  return (
                    <div class={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
                      <div class="max-w-[75%]">
                        <Show when={!isUser}>
                          <div class="text-xs font-semibold mb-1 text-[#8fa0be]">
                            {isPM ? 'PM' : isPO ? 'PO' : msg.agent_type ?? 'Agent'}
                          </div>
                        </Show>
                        <Show when={isStructuredQuestion && !isUser}>
                          {(() => {
                            try {
                              const q = JSON.parse(msg.content) as StructuredQuestion;
                              return (
                                <div class="space-y-2">
                                  <StructuredQuestionWidget
                                    messageId={msg.id}
                                    sessionId={msg.session_id}
                                    question={q}
                                    answered={isAnswered()}
                                  />
                                  <Show when={isAnswered() && msg.id > 0}>
                                    <div class="rounded-2xl px-4 py-2 text-sm leading-relaxed bg-blue-50 text-[#2f3f5f] border border-[#c7d6f0]">
                                      Selected: {(answeredResponseForQuestion(msg.id)?.selected ?? []).join(', ')}
                                    </div>
                                  </Show>
                                </div>
                              );
                            } catch {
                              return <span class="text-xs text-red-400">[invalid question]</span>;
                            }
                          })()}</Show>
                        <Show when={!isStructuredQuestion || isUser}>
                          <div
                            class={`rounded-2xl px-4 py-2 text-sm leading-relaxed whitespace-pre-wrap ${
                              isUser
                                ? 'bg-blue-500 text-white rounded-br-md'
                                : isPM
                                  ? 'bg-gray-100 text-[#2f3f5f] rounded-bl-md'
                                  : isPO
                                    ? 'bg-purple-100 text-[#2f3f5f] rounded-bl-md'
                                    : 'bg-gray-100 text-[#2f3f5f] rounded-bl-md'
                            }`}
                          >
                            {isStructuredResponse ? responseLabel() : msg.content}
                          </div>
                        </Show>
                      </div>
                    </div>
                  );
                }}
              </For>
            </Show>
            <Show when={urlSessionId() === null}>
              <div class="flex items-center justify-center h-full text-[#8fa0be] text-sm">
                Select a session or start a new chat
              </div>
            </Show>
          </div>

          <div class="border-t border-[#c6cfdf] p-3 bg-[#f6f8fe]">
            <Show when={urlSessionId() !== null && unansweredQuestionCount() > 0}>
              <div class="mb-3 card bg-base-100 border border-base-300 shadow-sm">
                <div class="card-body p-3">
                  <div class="text-xs text-base-content/70 mb-2">
                  {unansweredQuestionCount()} unanswered question{unansweredQuestionCount() === 1 ? '' : 's'}
                  </div>
                  <div class="space-y-2 max-h-56 overflow-y-auto pr-1">
                    <For each={unansweredQuestionRows()}>
                      {(qMsg) => {
                        const isPM = qMsg.agent_type === 'project_manager';
                        const isPO = qMsg.agent_type === 'product_owner';
                        return (
                          <div class="space-y-1">
                            <div class="text-xs font-semibold text-base-content/50">
                              {isPM ? 'PM' : isPO ? 'PO' : qMsg.agent_type ?? 'Agent'}
                            </div>
                            {(() => {
                              try {
                                const q = JSON.parse(qMsg.content) as StructuredQuestion;
                                return (
                                  <StructuredQuestionWidget
                                    messageId={qMsg.id}
                                    sessionId={qMsg.session_id}
                                    question={q}
                                    answered={false}
                                  />
                                );
                              } catch {
                                return <span class="text-xs text-error">[invalid question]</span>;
                              }
                            })()}
                          </div>
                        );
                      }}
                    </For>
                  </div>
                </div>
              </div>
            </Show>
            <div class="flex gap-2">
              <textarea
                class="textarea textarea-bordered textarea-sm flex-1 min-h-[40px] resize-none"
                placeholder={
                  urlSessionId() === null
                    ? 'Start a new conversation...'
                    : 'Type your message...'
                }
                value={inputText()}
                onInput={(e) => setInputText(e.currentTarget.value)}
                disabled={currentSession()?.status === 'completed' || chat.loading()}
                onKeyDown={handleKeyDown}
              />
              <button
                class="btn btn-primary self-end"
                onClick={() => void handleSend()}
                disabled={
                  !inputText().trim() ||
                  currentSession()?.status === 'completed' ||
                  chat.loading()
                }
              >
                Send
              </button>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}

export default function UserChatPage() {
  const params = useParams<{ projectId: string }>();

  return (
    <UserChatProvider>
      <UserChatInner projectId={() => params.projectId ? parseInt(params.projectId) : undefined} />
    </UserChatProvider>
  );
}

function UserChatInner(props: { projectId: () => number | undefined }) {
  const params = useParams<{ projectId: string; sessionId?: string }>();
  const navigate = useNavigate();
  const chat = useUserChat();

  // Load sessions when project is available
  createEffect(() => {
    const id = props.projectId();
    if (id) void chat.loadSessions(id);
  });

  // Once sessions load and no sessionId in URL, redirect to most recent
  createEffect(() => {
    const sessions = chat.sessions();
    if (sessions.length > 0 && !params.sessionId) {
      const mostRecent = sessions.reduce((a, b) =>
        new Date(a.created_at) > new Date(b.created_at) ? a : b
      );
      navigate(`/projects/${params.projectId}/chat/${mostRecent.id}`, { replace: true });
    }
  });

  // When sessionId in URL changes, sync context selection and load messages
  createEffect(() => {
    const sid = params.sessionId ? parseInt(params.sessionId) : null;
    void chat.selectSession(sid);
  });

  // When PO hands off to PM, navigate to the new planning session automatically
  createEffect(() => {
    const hid = chat.handoffSessionId();
    if (hid) {
      void chat.loadSessions(props.projectId()!);
      navigate(`/projects/${params.projectId}/chat/${hid}`);
    }
  });

  return <ChatContent projectId={props.projectId} />;
}
