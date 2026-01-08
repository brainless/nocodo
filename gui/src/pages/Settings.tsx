import { createSignal, onMount, type Component, For } from 'solid-js';
import type {
  SettingsResponse,
  ApiKeyConfig,
  UpdateApiKeysRequest,
} from '../../api-types/types';

const Settings: Component = () => {
  const [settings, setSettings] = createSignal<SettingsResponse | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [success, setSuccess] = createSignal<string | null>(null);

  // Store form values
  const [formValues, setFormValues] = createSignal<
    Record<string, string | undefined>
  >({});

  onMount(async () => {
    try {
      const response = await fetch('http://127.0.0.1:8080/settings');
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data: SettingsResponse = await response.json();
      setSettings(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to fetch settings');
    } finally {
      setLoading(false);
    }
  });

  const handleInputChange = (name: string, value: string) => {
    setFormValues((prev) => ({
      ...prev,
      [name]: value,
    }));
  };

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    setSuccess(null);

    try {
      const values = formValues();
      const requestData: UpdateApiKeysRequest = {
        xai_api_key: values['xai'] || null,
        openai_api_key: values['openai'] || null,
        anthropic_api_key: values['anthropic'] || null,
        zai_api_key: values['zai'] || null,
        zai_coding_plan: null,
      };

      const response = await fetch('http://127.0.0.1:8080/settings/api-keys', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(requestData),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      setSuccess('API keys updated successfully!');
      // Clear form values
      setFormValues({});
      // Refresh settings
      const settingsResponse = await fetch('http://127.0.0.1:8080/settings');
      if (settingsResponse.ok) {
        const data: SettingsResponse = await settingsResponse.json();
        setSettings(data);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update API keys');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="container mx-auto p-8 max-w-4xl">
      <h1 class="text-3xl font-bold mb-6">Settings</h1>

      {loading() && (
        <div class="flex justify-center">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
      )}

      {error() && (
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
      )}

      {success() && (
        <div class="alert alert-success mb-6">
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
          <span>{success()}</span>
        </div>
      )}

      {!loading() && settings() && (
        <div class="card bg-base-100 shadow-xl">
          <div class="card-body">
            <h2 class="card-title mb-4">API Keys</h2>
            <p class="text-sm text-base-content/70 mb-6">
              Configure your API keys for different LLM providers. Keys are
              stored securely and masked for security.
            </p>

            <form onSubmit={handleSubmit}>
              <div class="space-y-6">
                <For each={settings()?.api_keys}>
                  {(apiKey: ApiKeyConfig) => (
                    <div class="form-control">
                      <label class="label">
                        <span class="label-text font-medium capitalize">
                          {apiKey.name} API Key
                        </span>
                        {apiKey.is_configured && (
                          <span class="label-text-alt">
                            <div class="badge badge-success badge-sm">
                              Configured
                            </div>
                          </span>
                        )}
                      </label>
                      <input
                        type="password"
                        placeholder={
                          apiKey.is_configured
                            ? apiKey.key || 'Enter new key to update'
                            : `Enter ${apiKey.name} API key`
                        }
                        class="input input-bordered w-full"
                        value={formValues()[apiKey.name] || ''}
                        onInput={(e) =>
                          handleInputChange(apiKey.name, e.currentTarget.value)
                        }
                      />
                      <label class="label">
                        <span class="label-text-alt text-base-content/60">
                          {apiKey.is_configured
                            ? 'Leave blank to keep current key'
                            : 'Required for using this provider'}
                        </span>
                      </label>
                    </div>
                  )}
                </For>
              </div>

              <div class="card-actions justify-end mt-6">
                <button
                  type="submit"
                  class="btn btn-primary"
                  disabled={saving()}
                >
                  {saving() ? (
                    <>
                      <span class="loading loading-spinner loading-sm"></span>
                      Saving...
                    </>
                  ) : (
                    'Save API Keys'
                  )}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {!loading() && !settings() && (
        <div class="text-center text-base-content/70">
          <p>Unable to load settings</p>
        </div>
      )}
    </div>
  );
};

export default Settings;
