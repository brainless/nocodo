import { Component, For, Show, createSignal, onMount } from 'solid-js';
import { apiClient } from '../api';
import type { ApiKeyConfig, SettingsResponse } from '../types';

const Settings: Component = () => {
  const [settings, setSettings] = createSignal<SettingsResponse | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(() => {
    const fetchSettings = async () => {
      try {
        console.log('Fetching settings...');
        const response = await apiClient.getSettings();
        console.log('Settings response:', response);
        setSettings(response);
        console.log('Settings set, loading should be false now');
      } catch (err) {
        console.error('Settings error:', err);
        setError(err instanceof Error ? err.message : 'Failed to load settings');
      } finally {
        console.log('Setting loading to false');
        setLoading(false);
        console.log('Loading state after setting to false:', loading());
      }
    };

    fetchSettings();
  });

  const getStatusBadge = (apiKey: ApiKeyConfig) => {
    if (apiKey.is_configured) {
      return (
        <span class='inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800'>
          Configured
        </span>
      );
    } else {
      return (
        <span class='inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-red-100 text-red-800'>
          Not Configured
        </span>
      );
    }
  };

  const formatApiKey = (apiKey: ApiKeyConfig) => {
    if (!apiKey.is_configured) {
      return 'Not configured';
    }
    return apiKey.key || '****';
  };

  console.log(
    'Settings component render - loading:',
    loading(),
    'error:',
    error(),
    'settings:',
    settings()
  );

  return (
    <div>
      <Show when={loading()}>
        <div class='flex justify-center items-center h-64'>
          <div class='text-gray-500'>Loading settings...</div>
        </div>
      </Show>

      <Show when={error()}>
        <div class='bg-red-50 border border-red-200 rounded-md p-4'>
          <div class='text-red-800'>
            <strong>Error:</strong> {error()}
          </div>
        </div>
      </Show>

      {!loading() && !error() && settings() && (() => {
        const settingsData = settings()!;
        console.log('Rendering main settings UI with data:', settingsData);
        return (
          <div class='space-y-8'>
            {/* Instructions */}
            <div class='bg-blue-50 border border-blue-200 rounded-lg p-6'>
              <h3 class='text-lg font-medium text-blue-900 mb-3'>API Key Configuration</h3>
              <div class='text-blue-800 space-y-2'>
                <p>
                  Configure your AI provider API keys in the configuration file to enable
                  AI-assisted development features.
                </p>
                <p>
                  <strong>Configuration file location:</strong>
                  <code class='ml-2 px-2 py-1 bg-blue-100 rounded text-sm font-mono'>
                    {settingsData.config_file_path}
                  </code>
                </p>
              </div>
            </div>

            {/* Configuration Instructions */}
            <div class='bg-gray-50 border border-gray-200 rounded-lg p-6'>
              <h3 class='text-lg font-medium text-gray-900 mb-3'>How to Configure API Keys</h3>
              <div class='text-gray-700 space-y-3'>
                <p>
                  Add your API keys to the{' '}
                  <code class='px-1 py-0.5 bg-gray-200 rounded text-sm'>[api_keys]</code> section
                  in your configuration file:
                </p>
                <pre class='bg-gray-800 text-gray-100 p-4 rounded-md text-sm overflow-x-auto'>
                  {`[api_keys]
grok_api_key = "your-grok-api-key-here"
openai_api_key = "your-openai-api-key-here"
anthropic_api_key = "your-anthropic-api-key-here"`}
                </pre>
                <p class='text-sm text-gray-600'>
                  <strong>Note:</strong> Restart the nocodo manager after updating the
                  configuration file for changes to take effect.
                </p>
              </div>
            </div>

            {/* API Keys Status */}
            <div class='bg-white border border-gray-200 rounded-lg'>
              <div class='px-6 py-4 border-b border-gray-200'>
                <h3 class='text-lg font-medium text-gray-900'>API Keys Status</h3>
                <p class='text-sm text-gray-600 mt-1'>
                  Current configuration status for AI provider API keys
                </p>
              </div>
              <div class='divide-y divide-gray-200'>
                <For each={settingsData.api_keys}>
                  {apiKey => (
                    <div class='px-6 py-4 flex items-center justify-between'>
                      <div class='flex-1'>
                        <div class='flex items-center space-x-3'>
                          <h4 class='text-sm font-medium text-gray-900'>{apiKey.name}</h4>
                          {getStatusBadge(apiKey)}
                        </div>
                        <div class='mt-1 text-sm text-gray-600 font-mono'>
                          {formatApiKey(apiKey)}
                        </div>
                      </div>
                      <div class='text-right'>
                        {apiKey.is_configured ? (
                          <svg
                            class='w-5 h-5 text-green-500'
                            fill='currentColor'
                            viewBox='0 0 20 20'
                          >
                            <path
                              fill-rule='evenodd'
                              d='M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z'
                              clip-rule='evenodd'
                            />
                          </svg>
                        ) : (
                          <svg
                            class='w-5 h-5 text-red-500'
                            fill='currentColor'
                            viewBox='0 0 20 20'
                          >
                            <path
                              fill-rule='evenodd'
                              d='M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z'
                              clip-rule='evenodd'
                            />
                          </svg>
                        )}
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </div>

            {/* Additional Information */}
            <div class='bg-yellow-50 border border-yellow-200 rounded-lg p-6'>
              <h3 class='text-lg font-medium text-yellow-900 mb-3'>Provider Information</h3>
              <div class='text-yellow-800 space-y-2 text-sm'>
                <p>
                  <strong>Grok:</strong> Get your API key from{' '}
                  <a
                    href='https://console.x.ai/'
                    target='_blank'
                    rel='noopener noreferrer'
                    class='underline'
                  >
                    X.AI Console
                  </a>
                </p>
                <p>
                  <strong>OpenAI:</strong> Get your API key from{' '}
                  <a
                    href='https://platform.openai.com/api-keys'
                    target='_blank'
                    rel='noopener noreferrer'
                    class='underline'
                  >
                    OpenAI Platform
                  </a>
                </p>
                <p>
                  <strong>Anthropic:</strong> Get your API key from{' '}
                  <a
                    href='https://console.anthropic.com/'
                    target='_blank'
                    rel='noopener noreferrer'
                    class='underline'
                  >
                    Anthropic Console
                  </a>
                </p>
              </div>
            </div>
          </div>
        );
      })()}
    </div>
  );
};

export default Settings;
