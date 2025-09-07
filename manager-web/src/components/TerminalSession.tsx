import { Component, createSignal, onCleanup } from 'solid-js';
import { Terminal as XTerm } from '@xterm/xterm';
import { apiClient } from '../api';
import Terminal from './Terminal';

export interface TerminalSessionProps {
  sessionId: string;
  className?: string;
}

interface TerminalConnection {
  close: () => void;
}

const TerminalSession: Component<TerminalSessionProps> = props => {
  let terminal: XTerm | undefined;
  let connection: TerminalConnection | undefined;

  const [connectionStatus, setConnectionStatus] = createSignal<
    'connecting' | 'connected' | 'disconnected' | 'error'
  >('connecting');
  const [error, setError] = createSignal<string | null>(null);

  const connectToSession = () => {
    try {
      console.log(`Connecting to terminal session: ${props.sessionId}`);
      setConnectionStatus('connecting');
      setError(null);

      connection = apiClient.subscribeTerminal(
        props.sessionId,
        // Binary data handler (terminal output)
        (data: ArrayBuffer) => {
          if (terminal) {
            const uint8Array = new Uint8Array(data);
            terminal.write(uint8Array);
          }
        },
        // Control message handler
        (message: any) => {
          console.log('Received control message:', message);

          switch (message.type) {
            case 'status':
              console.log(`Session status: ${message.status}`, message.exit_code);
              if (message.status === 'completed' || message.status === 'failed') {
                setConnectionStatus('disconnected');
                if (terminal && message.status === 'completed') {
                  terminal.write('\r\n\x1b[32m--- Session completed ---\x1b[0m\r\n');
                } else if (terminal && message.status === 'failed') {
                  const exitCode = message.exit_code ?? 'unknown';
                  terminal.write(
                    `\r\n\x1b[31m--- Session failed (exit code: ${exitCode}) ---\x1b[0m\r\n`
                  );
                }
              }
              break;

            case 'pong':
              // Handle ping/pong for keepalive
              break;

            case 'resize':
              // Terminal was resized on server side
              if (terminal && message.cols && message.rows) {
                terminal.resize(message.cols, message.rows);
              }
              break;

            default:
              console.log('Unknown control message type:', message.type);
          }
        },
        // Error handler
        (error: Error) => {
          console.error('Terminal WebSocket error:', error);
          setConnectionStatus('error');
          setError(error.message);

          if (terminal) {
            terminal.write(`\r\n\x1b[31mConnection error: ${error.message}\x1b[0m\r\n`);
          }
        },
        // Open handler
        () => {
          console.log('Terminal WebSocket connected');
          setConnectionStatus('connected');
          setError(null);

          if (terminal) {
            terminal.write('\x1b[32m--- Connected to interactive session ---\x1b[0m\r\n');
            terminal.focus();
          }
        },
        // Close handler
        () => {
          console.log('Terminal WebSocket disconnected');
          setConnectionStatus('disconnected');

          if (terminal) {
            terminal.write('\r\n\x1b[33m--- Connection closed ---\x1b[0m\r\n');
          }
        }
      );
    } catch (err) {
      console.error('Failed to connect to terminal session:', err);
      setConnectionStatus('error');
      setError(err instanceof Error ? err.message : 'Connection failed');
    }
  };

  const handleTerminalReady = (term: XTerm) => {
    terminal = term;
    console.log('Terminal ready, establishing connection...');
    connectToSession();
  };

  const handleTerminalInput = (data: string) => {
    if (connectionStatus() === 'connected') {
      // Encode input as base64 for the control message
      const encoded = btoa(data);

      try {
        // Send input via WebSocket control message
        // This should be handled by the WebSocket connection, but we can fall back to HTTP
        apiClient.sendTerminalInput(props.sessionId, encoded).catch(err => {
          console.warn('Failed to send input via HTTP fallback:', err);
        });
      } catch (err) {
        console.error('Failed to send terminal input:', err);
      }
    }
  };

  const handleTerminalResize = (cols: number, rows: number) => {
    if (connectionStatus() === 'connected') {
      try {
        // Send resize via HTTP endpoint
        apiClient.resizeTerminal(props.sessionId, cols, rows).catch(err => {
          console.warn('Failed to resize terminal:', err);
        });
      } catch (err) {
        console.error('Failed to resize terminal:', err);
      }
    }
  };

  // Cleanup connection when component unmounts
  onCleanup(() => {
    if (connection) {
      console.log('Cleaning up terminal connection');
      connection.close();
    }
  });

  // Connection status indicator
  const getStatusIndicator = () => {
    switch (connectionStatus()) {
      case 'connecting':
        return { color: 'text-yellow-500', text: 'Connecting...', icon: '⚡' };
      case 'connected':
        return { color: 'text-green-500', text: 'Connected', icon: '✓' };
      case 'disconnected':
        return { color: 'text-gray-500', text: 'Disconnected', icon: '○' };
      case 'error':
        return { color: 'text-red-500', text: 'Error', icon: '✗' };
      default:
        return { color: 'text-gray-500', text: 'Unknown', icon: '?' };
    }
  };

  const statusInfo = getStatusIndicator();

  return (
    <div class={`terminal-session ${props.className || ''}`}>
      {/* Connection status bar */}
      <div class='bg-gray-100 border border-gray-300 rounded-t px-4 py-2 flex items-center justify-between text-sm'>
        <div class='flex items-center space-x-2'>
          <span class='font-medium text-gray-700'>Interactive Terminal</span>
          <span class='text-gray-500'>({props.sessionId.slice(-8)})</span>
        </div>
        <div class={`flex items-center space-x-1 ${statusInfo.color}`}>
          <span>{statusInfo.icon}</span>
          <span class='font-medium'>{statusInfo.text}</span>
        </div>
      </div>

      {/* Error display */}
      {error() && (
        <div class='bg-red-100 border-l-4 border-red-500 p-4'>
          <div class='flex'>
            <div class='flex-shrink-0'>
              <span class='text-red-500'>✗</span>
            </div>
            <div class='ml-3'>
              <p class='text-sm text-red-700'>
                <strong>Connection Error:</strong> {error()}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Terminal component */}
      <div class='relative border-l border-r border-b border-gray-300 rounded-b'>
        <Terminal
          sessionId={props.sessionId}
          onTerminalReady={handleTerminalReady}
          onInput={handleTerminalInput}
          onResize={handleTerminalResize}
          className='h-96'
        />
      </div>

      {/* Enhanced Terminal toolbar */}
      <div class='bg-gray-50 border-l border-r border-b border-gray-300 rounded-b px-4 py-2'>
        <div class='flex items-center justify-between text-sm'>
          <div class='flex items-center space-x-4'>
            <span class='text-gray-600'>Terminal Controls:</span>
            <div class='flex items-center space-x-1 text-xs text-gray-500'>
              <kbd class='px-1 py-0.5 bg-gray-200 rounded'>Ctrl+C</kbd>
              <span>interrupt</span>
              <span class='mx-2'>•</span>
              <kbd class='px-1 py-0.5 bg-gray-200 rounded'>Ctrl+D</kbd>
              <span>exit</span>
            </div>
          </div>
          <div class='flex items-center space-x-2'>
            {/* Copy terminal content */}
            <button
              onClick={async () => {
                if (terminal) {
                  try {
                    const selection = terminal.getSelection();
                    const content = selection || 'No text selected';
                    await navigator.clipboard.writeText(content);
                    console.log('Copied to clipboard');
                  } catch (err) {
                    console.error('Failed to copy:', err);
                  }
                }
              }}
              class='px-2 py-1 bg-gray-200 hover:bg-gray-300 rounded text-xs font-medium flex items-center space-x-1'
              disabled={connectionStatus() !== 'connected'}
              title='Copy selected text or current line'
            >
              <span>📋</span>
              <span>Copy</span>
            </button>

            {/* Paste from clipboard */}
            <button
              onClick={async () => {
                if (terminal && connectionStatus() === 'connected') {
                  try {
                    const text = await navigator.clipboard.readText();
                    handleTerminalInput(text);
                  } catch (err) {
                    console.error('Failed to paste:', err);
                  }
                }
              }}
              class='px-2 py-1 bg-gray-200 hover:bg-gray-300 rounded text-xs font-medium flex items-center space-x-1'
              disabled={connectionStatus() !== 'connected'}
              title='Paste from clipboard'
            >
              <span>📄</span>
              <span>Paste</span>
            </button>

            {/* Clear terminal */}
            <button
              onClick={() => terminal?.clear()}
              class='px-2 py-1 bg-gray-200 hover:bg-gray-300 rounded text-xs font-medium flex items-center space-x-1'
              disabled={connectionStatus() !== 'connected'}
              title='Clear terminal screen'
            >
              <span>🧹</span>
              <span>Clear</span>
            </button>

            {/* Send Ctrl+C */}
            <button
              onClick={() => {
                if (terminal && connectionStatus() === 'connected') {
                  handleTerminalInput('\x03');
                }
              }}
              class='px-2 py-1 bg-red-100 hover:bg-red-200 text-red-700 rounded text-xs font-medium flex items-center space-x-1'
              disabled={connectionStatus() !== 'connected'}
              title='Send interrupt signal (Ctrl+C)'
            >
              <span>⚡</span>
              <span>Ctrl+C</span>
            </button>

            {/* Send Ctrl+D */}
            <button
              onClick={() => {
                if (terminal && connectionStatus() === 'connected') {
                  handleTerminalInput('\x04');
                }
              }}
              class='px-2 py-1 bg-yellow-100 hover:bg-yellow-200 text-yellow-700 rounded text-xs font-medium flex items-center space-x-1'
              disabled={connectionStatus() !== 'connected'}
              title='Send end-of-file signal (Ctrl+D)'
            >
              <span>🔚</span>
              <span>Ctrl+D</span>
            </button>

            {/* Font size controls */}
            <div class='flex items-center space-x-1'>
              <button
                onClick={() => {
                  if (terminal) {
                    const currentSize = (terminal.options as any).fontSize || 14;
                    (terminal.options as any).fontSize = Math.max(8, currentSize - 1);
                  }
                }}
                class='px-2 py-1 bg-gray-200 hover:bg-gray-300 rounded text-xs font-medium'
                title='Decrease font size'
              >
                A-
              </button>
              <button
                onClick={() => {
                  if (terminal) {
                    const currentSize = (terminal.options as any).fontSize || 14;
                    (terminal.options as any).fontSize = Math.min(24, currentSize + 1);
                  }
                }}
                class='px-2 py-1 bg-gray-200 hover:bg-gray-300 rounded text-xs font-medium'
                title='Increase font size'
              >
                A+
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default TerminalSession;
