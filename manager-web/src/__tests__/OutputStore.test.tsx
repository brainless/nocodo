import { describe, it, expect, vi, beforeEach } from 'vitest';
import { SessionsProvider, useSessions } from '../stores/sessionsStore';
import { apiClient } from '../api';
import { render } from '@solidjs/testing-library';
import { Component, createEffect } from 'solid-js';

vi.mock('../api', () => {
  return {
    apiClient: {
      listAiOutputs: vi.fn().mockResolvedValue({ outputs: [
        { id: 1, session_id: 's1', content: 'hello', created_at: 1 },
        { id: 2, session_id: 's1', content: ' world', created_at: 2 },
      ]}),
      subscribeSession: vi.fn().mockImplementation((_id: string, onMessage: (d: any) => void) => {
        setTimeout(() => onMessage({ type: 'AiSessionOutputChunk', payload: { session_id: 's1', content: '!', stream: 'stdout', seq: 2 } }), 0);
        return { close: () => {} };
      }),
      getSession: vi.fn().mockResolvedValue({ id: 's1', status: 'running', tool_name: 'echo', prompt: '', started_at: 1 }),
    }
  };
});

const Harness: Component<{ onReady: (getOutputs: () => string) => void }> = (props) => {
  const { actions } = useSessions();
  createEffect(() => {
    actions.fetchOutputs('s1').then(() => {
      actions.connect('s1');
      props.onReady(() => actions.getOutputs('s1').map(c => c.content).join(''));
    });
  });
  return null as any;
};

describe('sessions store outputs', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('buffers outputs from HTTP and WS', async () => {
    let get: (() => string) | null = null;
    render(() => (
      <SessionsProvider>
        <Harness onReady={(fn) => (get = fn)} />
      </SessionsProvider>
    ));

    // wait a tick for WS message
    await new Promise(r => setTimeout(r, 10));

    expect(get).toBeTruthy();
    expect(get!()).toContain('hello');
    expect(get!()).toContain(' world');
    expect(get!()).toContain('!');
  });
});
