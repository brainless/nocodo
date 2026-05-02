import { For, Show, type JSX } from 'solid-js';
import { useChat, type UiMessage } from '../contexts/ChatContext';
import { PromptBox } from './PromptBox';

interface ChatDrawerProps {
  children: JSX.Element;
  renderMessage?: (msg: UiMessage) => JSX.Element;
  drawerId?: string;
  placeholder?: string | (() => string);
}

const AGENT_CONTACTS = [
  { type: 'project_manager', name: 'Project Manager', initial: 'PM' },
  { type: 'schema_designer', name: 'Database Dev', initial: 'DB' },
  { type: 'backend_developer', name: 'Backend Developer', initial: 'BE' },
  { type: 'frontend_developer', name: 'UI Designer', initial: 'UI' },
];

export default function ChatDrawer(props: ChatDrawerProps) {
  const chat = useChat();
  const drawerId = props.drawerId ?? 'chat-drawer';

  const selectedAgentName = () =>
    AGENT_CONTACTS.find((c) => c.type === chat.selectedAgentType())?.name ?? 'AI Assistant';

  const defaultRenderMessage = (msg: UiMessage) => (
    <div class={`chat ${msg.role === 'user' ? 'chat-end' : 'chat-start'}`}>
      <div
        class={`chat-bubble whitespace-pre-wrap ${msg.role === 'user' ? 'chat-bubble-primary' : 'bg-transparent'}`}
      >
        {msg.content}
      </div>
    </div>
  );

  const render = props.renderMessage ?? defaultRenderMessage;

  const placeholder = () => {
    if (typeof props.placeholder === 'function') return props.placeholder();
    if (typeof props.placeholder === 'string') return props.placeholder;
    return 'Type a message...';
  };

  return (
    <div class="drawer">
      <input id={drawerId} type="checkbox" class="drawer-toggle" />

      <div class="drawer-content flex flex-col">{props.children}</div>

      <div class="drawer-side z-50">
        <label for={drawerId} aria-label="close sidebar" class="drawer-overlay" />
        <div class="chat-sidebar">
          <div class="agent-list">
            <div class="chat-panel-header">
              <h3 class="text-sm font-semibold">Dev Team</h3>
              <label for={drawerId} class="btn btn-ghost btn-sm btn-square">
                ✕
              </label>
            </div>
            <div class="flex-1 overflow-y-auto">
              <ul class="list bg-base-100">
                <For each={AGENT_CONTACTS}>
                  {(contact) => {
                    const isSelected = () => chat.selectedAgentType() === contact.type;
                    return (
                      <li
                        class={`list-row items-center cursor-pointer ${isSelected() ? 'bg-base-200' : 'hover:bg-base-200/50'}`}
                        onClick={() => chat.selectAgent(contact.type)}
                      >
                        <div class="avatar placeholder">
                          <div
                            class={`text-neutral-content w-10 h-10 rounded-full flex items-center justify-center text-sm font-bold ${isSelected() ? 'bg-primary' : 'bg-neutral'}`}
                          >
                            {contact.initial}
                          </div>
                        </div>
                        <div class="list-col-grow">
                          <p class="text-sm font-medium">{contact.name}</p>
                          <p class="text-xs text-base-content/50">Online</p>
                        </div>
                      </li>
                    );
                  }}
                </For>
              </ul>
            </div>
          </div>

          <div class="chat-panel">
            <div class="chat-panel-header">
              <p class="text-sm font-semibold flex-1 truncate">{selectedAgentName()}</p>
              <label for={drawerId} class="btn btn-ghost btn-sm btn-square">
                ✕
              </label>
            </div>

            <div class="chat-messages">
              <Show when={chat.chatLoading() && chat.messages().length <= 1}>
                <div class="flex justify-center p-4">
                  <span class="loading loading-spinner loading-sm" />
                </div>
              </Show>
              <For each={chat.messages()}>{(msg) => render(msg)}</For>
              <Show when={chat.chatLoading() && chat.messages().length > 1}>
                <div class="chat chat-start">
                  <div class="chat-bubble">
                    <span class="loading loading-dots loading-xs" />
                  </div>
                </div>
              </Show>
            </div>

            <div class="chat-input-area">
              <PromptBox
                placeholder={placeholder()}
                submitLabel="Send"
                onSubmit={chat.sendMessage}
                loading={chat.chatLoading()}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
