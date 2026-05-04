import { createContext, createSignal, JSX, onMount, useContext } from 'solid-js';

type AuthState = {
  checking: boolean;
  authRequired: boolean;
  isAuthenticated: boolean;
  email: string | null;
};

type AuthContextValue = {
  auth: () => AuthState;
  onAuthenticated: () => Promise<void>;
  onLogout: () => Promise<void>;
};

const AuthContext = createContext<AuthContextValue>();

export function AuthProvider(props: { children: JSX.Element }) {
  const [auth, setAuth] = createSignal<AuthState>({
    checking: true,
    authRequired: false,
    isAuthenticated: false,
    email: null,
  });

  onMount(async () => {
    try {
      const hbRes = await fetch('/api/heartbeat').catch(() => null);
      if (!hbRes?.ok) {
        setAuth({ checking: false, authRequired: false, isAuthenticated: true, email: null });
        return;
      }
      const hb = await hbRes.json();
      const authRequired: boolean = hb.auth_required ?? false;

      if (!authRequired) {
        setAuth({ checking: false, authRequired: false, isAuthenticated: true, email: null });
        return;
      }

      const meRes = await fetch('/api/auth/me').catch(() => null);
      if (meRes?.ok) {
        const me = await meRes.json();
        setAuth({ checking: false, authRequired: true, isAuthenticated: true, email: me.email });
      } else {
        setAuth({ checking: false, authRequired: true, isAuthenticated: false, email: null });
      }
    } catch {
      setAuth({ checking: false, authRequired: false, isAuthenticated: true, email: null });
    }
  });

  const onAuthenticated = async () => {
    const meRes = await fetch('/api/auth/me').catch(() => null);
    const me = meRes?.ok ? await meRes.json() : null;
    setAuth(prev => ({ ...prev, isAuthenticated: true, email: me?.email ?? null }));
  };

  const onLogout = async () => {
    await fetch('/api/auth/logout', { method: 'POST' }).catch(() => null);
    setAuth(prev => ({ ...prev, isAuthenticated: false, email: null }));
  };

  return (
    <AuthContext.Provider value={{ auth, onAuthenticated, onLogout }}>
      {props.children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used within AuthProvider');
  return ctx;
}
