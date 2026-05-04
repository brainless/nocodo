import { createSignal, Show } from 'solid-js';
import { useAuth } from '../contexts/AuthContext';

export function AuthModal() {
  const { auth, onAuthenticated } = useAuth();
  const [step, setStep] = createSignal<'email' | 'otp'>('email');
  const [email, setEmail] = createSignal('');
  const [otp, setOtp] = createSignal('');
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const requestOtp = async (e: SubmitEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const res = await fetch('/api/auth/request-otp', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: email() }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error ?? 'Failed to send code');
        return;
      }
      setStep('otp');
    } catch {
      setError('Network error — please try again');
    } finally {
      setLoading(false);
    }
  };

  const verifyOtp = async (e: SubmitEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const res = await fetch('/api/auth/verify-otp', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: email(), otp: otp() }),
      });
      const data = await res.json();
      if (!res.ok) {
        setError(data.error ?? 'Invalid code');
        return;
      }
      await onAuthenticated();
    } catch {
      setError('Network error — please try again');
    } finally {
      setLoading(false);
    }
  };

  const goBack = () => {
    setStep('email');
    setOtp('');
    setError(null);
  };

  return (
    <Show when={auth().authRequired && !auth().isAuthenticated}>
      <div class="modal modal-open">
        <div class="modal-box max-w-sm">
          <h2 class="text-xl font-semibold mb-1">Sign in</h2>
          <p class="text-sm text-base-content/60 mb-6">
            Enter your email to receive a one-time code.
          </p>

          <Show when={step() === 'email'}>
            <form onSubmit={requestOtp} class="flex flex-col gap-3">
              <label class="form-control w-full">
                <div class="label">
                  <span class="label-text">Email address</span>
                </div>
                <input
                  type="email"
                  class="input input-bordered w-full"
                  placeholder="you@example.com"
                  value={email()}
                  onInput={e => setEmail(e.currentTarget.value)}
                  required
                  autofocus
                />
              </label>
              <Show when={error()}>
                <div class="alert alert-error text-sm py-2">
                  <span>{error()}</span>
                </div>
              </Show>
              <button type="submit" class="btn btn-primary w-full" disabled={loading()}>
                {loading() ? <span class="loading loading-spinner loading-sm" /> : 'Send code'}
              </button>
            </form>
          </Show>

          <Show when={step() === 'otp'}>
            <form onSubmit={verifyOtp} class="flex flex-col gap-3">
              <p class="text-sm">
                A 6-digit code was sent to <strong>{email()}</strong>.
              </p>
              <label class="form-control w-full">
                <div class="label">
                  <span class="label-text">Verification code</span>
                </div>
                <input
                  type="text"
                  class="input input-bordered w-full tracking-widest text-center text-lg"
                  placeholder="000000"
                  value={otp()}
                  onInput={e => setOtp(e.currentTarget.value.replace(/\D/g, '').slice(0, 6))}
                  maxLength={6}
                  inputMode="numeric"
                  autocomplete="one-time-code"
                  required
                  autofocus
                />
              </label>
              <Show when={error()}>
                <div class="alert alert-error text-sm py-2">
                  <span>{error()}</span>
                </div>
              </Show>
              <button type="submit" class="btn btn-primary w-full" disabled={loading()}>
                {loading() ? <span class="loading loading-spinner loading-sm" /> : 'Sign in'}
              </button>
              <button type="button" class="btn btn-ghost btn-sm" onClick={goBack}>
                Use a different email
              </button>
            </form>
          </Show>
        </div>
      </div>
    </Show>
  );
}
