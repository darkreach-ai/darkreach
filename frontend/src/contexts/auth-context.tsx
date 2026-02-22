"use client";

/**
 * @module auth-context
 *
 * Supabase Auth context provider for the dashboard. Wraps the entire
 * app in `<AuthProvider>` to provide `useAuth()` hooks with:
 *
 * - `user` / `session` — Current Supabase Auth state
 * - `signIn()` / `signOut()` — Email/password authentication
 * - `loading` — True during initial session restoration
 *
 * `<AuthGuard>` gates all routes except `/login`, redirecting
 * unauthenticated users to the login page. Session state is managed
 * by Supabase's `onAuthStateChange` listener (handles refresh tokens
 * and tab-focus re-authentication automatically).
 */

import {
  createContext,
  useContext,
  useEffect,
  useState,
  useCallback,
  type ReactNode,
} from "react";
import { supabase } from "@/lib/supabase";
import { API_BASE } from "@/lib/format";
import type { User, Session } from "@supabase/supabase-js";

export type UserRole = "admin" | "operator";

interface AuthContextValue {
  user: User | null;
  session: Session | null;
  loading: boolean;
  role: UserRole | null;
  operatorId: string | null;
  signIn: (email: string, password: string) => Promise<string | null>;
  signOut: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const [loading, setLoading] = useState(true);
  const [role, setRole] = useState<UserRole | null>(null);
  const [operatorId, setOperatorId] = useState<string | null>(null);

  /** Fetch user profile via REST API (auto-provisions operator on first call). */
  const fetchProfile = useCallback(async (accessToken: string) => {
    try {
      const res = await fetch(`${API_BASE}/api/auth/me`, {
        headers: { Authorization: `Bearer ${accessToken}` },
      });
      if (!res.ok) {
        setRole("operator");
        setOperatorId(null);
        return;
      }
      const data = await res.json();
      setRole((data.role ?? "operator") as UserRole);
      setOperatorId(data.operator_id ?? null);
    } catch {
      // Fallback if API is unreachable
      setRole("operator");
      setOperatorId(null);
    }
  }, []);

  useEffect(() => {
    supabase.auth.getSession().then(({ data: { session } }) => {
      setSession(session);
      setUser(session?.user ?? null);
      if (session?.access_token) {
        fetchProfile(session.access_token).then(() => setLoading(false));
      } else {
        setLoading(false);
      }
    });

    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      setSession(session);
      setUser(session?.user ?? null);
      if (session?.access_token) {
        fetchProfile(session.access_token);
      } else {
        setRole(null);
        setOperatorId(null);
      }
    });

    return () => subscription.unsubscribe();
  }, [fetchProfile]);

  const signIn = useCallback(
    async (email: string, password: string): Promise<string | null> => {
      const { error } = await supabase.auth.signInWithPassword({
        email,
        password,
      });
      return error ? error.message : null;
    },
    []
  );

  const signOut = useCallback(async () => {
    await supabase.auth.signOut();
  }, []);

  return (
    <AuthContext.Provider value={{ user, session, loading, role, operatorId, signIn, signOut }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return ctx;
}

/** Guard component that redirects unauthenticated users to the login page content. */
export function AuthGuard({ children }: { children: ReactNode }) {
  const { user, loading } = useAuth();

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-muted-foreground text-sm">Loading...</div>
      </div>
    );
  }

  if (!user) {
    // Render nothing — layout will show the login page instead
    return null;
  }

  return <>{children}</>;
}

/** Guard component that restricts access to admin-only pages. */
export function RoleGuard({
  children,
  requiredRole = "admin",
}: {
  children: ReactNode;
  requiredRole?: UserRole;
}) {
  const { role, loading } = useAuth();

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-muted-foreground text-sm">Loading...</div>
      </div>
    );
  }

  if (role !== requiredRole) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-center">
          <h2 className="text-lg font-semibold mb-2">Access Denied</h2>
          <p className="text-muted-foreground text-sm">
            This page requires {requiredRole} privileges.
          </p>
        </div>
      </div>
    );
  }

  return <>{children}</>;
}
