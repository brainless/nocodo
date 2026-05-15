import { For, Show, createEffect, createSignal } from 'solid-js';
import { useProject } from '../contexts/ProjectContext';
import { UserChatProvider, useUserChat } from '../contexts/UserChatContext';

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

function ChatContent(props: { projectId: () => number | undefined }) {
  const chat = useUserChat();
  const [inputText, setInputText] = createSignal('');

  const pid = () => props.projectId();

  createEffect(() => {
    const id = pid();
    if (id) chat.loadSessions(id);
  });

  const currentSession = () =>
    chat.sessions().find(s => s.id === chat.currentSessionId()) ?? null;

  const canSend = () =>
    currentSession()?.status !== 'completed' && !chat.loading();

  const handleSend = async () => {
    const msg = inputText().trim();
    if (!msg) return;

    const sid = chat.currentSessionId();
    if (sid === null) {
      const id = pid();
      if (!id) return;
      setInputText('');
      await chat.startSession(id, msg);
      return;
    }

    if (!canSend()) return;
    setInputText('');
    await chat.sendMessage(sid, msg);
  };

  const handleNewChat = () => {
    chat.selectSession(null);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
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
                    session.id === chat.currentSessionId()
                      ? 'bg-[#dde9ff] border border-[#c7d6f0]'
                      : 'hover:bg-[#e7ecf8] border border-transparent'
                  }`}
                  onClick={() => chat.selectSession(session.id)}
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
            <Show when={chat.currentSessionId() !== null && chat.messages().length > 0}>
              <For each={chat.messages()}>
                {(msg) => {
                  const isUser = msg.author_type === 'user';
                  const isPM = msg.agent_type === 'project_manager';
                  const isPO = msg.agent_type === 'product_owner';
                  return (
                    <div class={`flex ${isUser ? 'justify-end' : 'justify-start'}`}>
                      <div class="max-w-[75%]">
                        <Show when={!isUser}>
                          <div class="text-xs font-semibold mb-1 text-[#8fa0be]">
                            {isPM ? 'PM' : isPO ? 'PO' : msg.agent_type ?? 'Agent'}
                          </div>
                        </Show>
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
                          {msg.content}
                        </div>
                      </div>
                    </div>
                  );
                }}
              </For>
            </Show>
            <Show when={chat.currentSessionId() === null}>
              <div class="flex items-center justify-center h-full text-[#8fa0be] text-sm">
                Select a session or start a new chat
              </div>
            </Show>
          </div>

          <div class="border-t border-[#c6cfdf] p-3 bg-[#f6f8fe]">
            <div class="flex gap-2">
              <textarea
                class="textarea textarea-bordered textarea-sm flex-1 min-h-[40px] resize-none"
                placeholder={
                  chat.currentSessionId() === null
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
                onClick={handleSend}
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
  const { currentProject } = useProject();

  return (
    <UserChatProvider>
      <UserChatInner projectId={() => currentProject()?.id} />
    </UserChatProvider>
  );
}

function UserChatInner(props: { projectId: () => number | undefined }) {
  const chat = useUserChat();

  if (!chat.displayName()) {
    return <NamePrompt />;
  }

  return <ChatContent projectId={props.projectId} />;
}
