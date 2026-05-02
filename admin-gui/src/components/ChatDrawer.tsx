import { For, Show, type JSX } from 'solid-js';
import { useChat, type UiMessage } from '../contexts/ChatContext';
import { PromptBox } from './PromptBox';

interface ChatDrawerProps {
  children: JSX.Element;
  renderMessage?: (msg: UiMessage) => JSX.Element;
  agentName?: string;
  drawerId?: string;
  placeholder?: string | (() => string);
}

export default function ChatDrawer(props: ChatDrawerProps) {
  const chat = useChat();
  const drawerId = props.drawerId ?? 'chat-drawer';
  const agentName = props.agentName ?? 'AI Assistant';

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
                <li class="list-row items-center cursor-default bg-base-200">
                  <div class="avatar placeholder">
                    <div class="bg-neutral text-neutral-content w-10 h-10 rounded-full flex items-center justify-center">
                      <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="20"
                        height="20"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      >
                        <ellipse cx="12" cy="5" rx="9" ry="3" />
                        <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
                        <path d="M3 12c0 1.66 4 3 9 3s9-1.34 9-3" />
                      </svg>
                    </div>
                  </div>
                  <div class="list-col-grow">
                    <p class="text-sm font-medium">{agentName}</p>
                    <p class="text-xs text-base-content/50">Online</p>
                  </div>
                </li>
              </ul>
            </div>
          </div>

          <div class="chat-panel">
            <div class="chat-panel-header">
              <p class="text-sm font-semibold flex-1 truncate">{agentName}</p>
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
