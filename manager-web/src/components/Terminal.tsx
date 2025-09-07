import { Component, createSignal, onCleanup, onMount } from 'solid-js';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import '@xterm/xterm/css/xterm.css';

export interface TerminalProps {
  sessionId: string;
  onTerminalReady?: (terminal: XTerm) => void;
  onInput?: (data: string) => void;
  onResize?: (cols: number, rows: number) => void;
  className?: string;
}

interface TerminalConnection {
  close: () => void;
}

const Terminal: Component<TerminalProps> = props => {
  let terminalElement: HTMLDivElement | undefined;
  let terminal: XTerm | undefined;
  let fitAddon: FitAddon | undefined;
  let connection: TerminalConnection | undefined;

  const [error, setError] = createSignal<string | null>(null);

  // Initialize terminal when component mounts
  onMount(() => {
    if (!terminalElement) return;

    try {
      // Create terminal instance
      terminal = new XTerm({
        cursorBlink: true,
        theme: {
          background: '#000000',
          foreground: '#ffffff',
          cursor: '#ffffff',
          selectionBackground: '#ffffff40',
          black: '#2e3436',
          red: '#cc0000',
          green: '#4e9a06',
          yellow: '#c4a000',
          blue: '#3465a4',
          magenta: '#75507b',
          cyan: '#06989a',
          white: '#d3d7cf',
          brightBlack: '#555753',
          brightRed: '#ef2929',
          brightGreen: '#8ae234',
          brightYellow: '#fce94f',
          brightBlue: '#729fcf',
          brightMagenta: '#ad7fa8',
          brightCyan: '#34e2e2',
          brightWhite: '#eeeeec',
        },
        fontFamily: '"Cascadia Code", "Fira Code", "Consolas", monospace',
        fontSize: 14,
        lineHeight: 1.2,
        allowTransparency: true,
        convertEol: true,
      });

      // Create and attach addons
      fitAddon = new FitAddon();
      terminal.loadAddon(fitAddon);
      terminal.loadAddon(new WebLinksAddon());

      // Open terminal in the DOM element
      terminal.open(terminalElement);

      // Fit terminal to container
      fitAddon.fit();

      // Handle input from user
      terminal.onData(data => {
        if (props.onInput) {
          props.onInput(data);
        }
      });

      // Handle terminal resize
      terminal.onResize(({ cols, rows }) => {
        if (props.onResize) {
          props.onResize(cols, rows);
        }
      });

      // Setup resize observer for container changes
      const resizeObserver = new ResizeObserver(() => {
        if (fitAddon) {
          fitAddon.fit();
        }
      });

      if (terminalElement.parentElement) {
        resizeObserver.observe(terminalElement.parentElement);
      }

      // Cleanup resize observer
      onCleanup(() => {
        resizeObserver.disconnect();
      });

      // Notify parent component that terminal is ready
      if (props.onTerminalReady) {
        props.onTerminalReady(terminal);
      }

      setError(null);
      console.log('Terminal initialized successfully');
    } catch (err) {
      console.error('Failed to initialize terminal:', err);
      setError(err instanceof Error ? err.message : 'Failed to initialize terminal');
    }
  });

  // Cleanup terminal when component unmounts
  onCleanup(() => {
    if (connection) {
      connection.close();
    }
    if (terminal) {
      terminal.dispose();
    }
  });

  // Focus terminal when clicked
  const handleClick = () => {
    if (terminal) {
      terminal.focus();
    }
  };

  // Handle window resize
  const handleResize = () => {
    if (fitAddon) {
      fitAddon.fit();
    }
  };

  onMount(() => {
    window.addEventListener('resize', handleResize);
  });

  onCleanup(() => {
    window.removeEventListener('resize', handleResize);
  });

  // Public methods for parent components
  const writeData = (data: string | Uint8Array) => {
    if (terminal) {
      terminal.write(data);
    }
  };

  const clear = () => {
    if (terminal) {
      terminal.clear();
    }
  };

  const resize = (cols: number, rows: number) => {
    if (terminal) {
      terminal.resize(cols, rows);
    }
  };

  const focus = () => {
    if (terminal) {
      terminal.focus();
    }
  };

  // Expose methods to parent via callback
  onMount(() => {
    if (props.onTerminalReady && terminal) {
      // Extend terminal with our custom methods
      const extendedTerminal = terminal as XTerm & {
        writeData: typeof writeData;
        clear: typeof clear;
        resize: typeof resize;
        focus: typeof focus;
      };

      extendedTerminal.writeData = writeData;
      extendedTerminal.clear = clear;
      extendedTerminal.resize = resize;
      extendedTerminal.focus = focus;

      props.onTerminalReady(extendedTerminal);
    }
  });

  return (
    <div class={`terminal-container ${props.className || ''}`}>
      {error() && (
        <div class='bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4'>
          <strong class='font-bold'>Terminal Error: </strong>
          <span class='block sm:inline'>{error()}</span>
        </div>
      )}

      <div
        ref={(el: HTMLDivElement) => (terminalElement = el)}
        onClick={handleClick}
        class='terminal-element w-full h-full min-h-[400px] bg-black rounded border border-gray-300 focus-within:border-blue-500'
        style={{
          cursor: 'text',
        }}
      />

      {false && (
        <div class='absolute inset-0 flex items-center justify-center bg-black bg-opacity-75 text-white'>
          <div class='text-center'>
            <div class='animate-spin rounded-full h-8 w-8 border-b-2 border-white mx-auto mb-4'></div>
            <p class='text-sm'>Connecting to terminal...</p>
          </div>
        </div>
      )}
    </div>
  );
};

export default Terminal;
