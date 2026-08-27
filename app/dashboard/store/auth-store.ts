import { create } from "zustand";
import type { User, Session, OAuthProvider } from "@/lib/auth-types";
import { api, ApiError } from "@/lib/api";
import { env } from "@/lib/env";

/**
 * Real API-backed auth state.
 *
 * - Session ownership lives on the Rust API: it sets/clears the httpOnly
 *   `session_token` cookie; this store never touches document.cookie.
 * - Actions keep the `{ error?: string }` return contract the auth pages
 *   were written against (message text comes from problem documents).
 */

type LoginResponse = { user: User; session: Session };
type SignupResponse = LoginResponse & { verificationToken?: string };
type MeResponse = { user: User; onboardingCompleted: boolean };

/** Payload for `POST /v1/auth/onboarding/complete`. */
export type CompleteOnboardingInput = {
  organization: {
    name: string;
    logoUrl?: string | null;
  };
  project: {
    name: string;
    description?: string | null;
    environment: "development" | "staging" | "production";
  };
};

function toErrorMessage(err: unknown): string {
  if (err instanceof ApiError) return err.message;
  if (err instanceof TypeError) {
    return "Cannot reach the server. Please make sure the API is running.";
  }
  return "Something went wrong. Please try again.";
}

type AuthState = {
  user: User | null;
  session: Session | null;
  isLoading: boolean;
  isAuthenticated: boolean;
  /** Server-derived: owns ≥1 org with ≥1 project (can skip onboarding). */
  onboardingCompleted: boolean;

  login: (
    email: string,
    password: string,
    rememberMe: boolean
  ) => Promise<{ error?: string }>;
  signup: (
    name: string,
    email: string,
    password: string
  ) => Promise<{ error?: string; verificationToken?: string }>;
  loginWithOAuth: (
    provider: OAuthProvider
  ) => Promise<{ error?: string } | undefined>;
  logout: () => Promise<void>;
  forgotPassword: (email: string) => Promise<{ error?: string }>;
  resetPassword: (
    token: string,
    password: string
  ) => Promise<{ error?: string }>;
  verifyEmail: (token: string) => Promise<{ error?: string }>;
  resendVerification: (email: string) => Promise<{ error?: string }>;
  fetchMe: () => Promise<void>;
  completeOnboarding: (
    input: CompleteOnboardingInput
  ) => Promise<{ error?: string; alreadyCompleted?: boolean }>;
};

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  session: null,
  isLoading: false,
  isAuthenticated: false,
  onboardingCompleted: false,

  login: async (email, password, rememberMe) => {
    set({ isLoading: true });

    try {
      const data = await api<LoginResponse>("/v1/auth/login", {
        method: "POST",
        body: JSON.stringify({ email, password, rememberMe }),
      });

      set({
        user: data.user,
        session: data.session,
        isLoading: false,
        isAuthenticated: true,
        onboardingCompleted: data.session.onboardingCompleted,
      });
      return {};
    } catch (err) {
      set({ isLoading: false });
      return { error: toErrorMessage(err) };
    }
  },

  signup: async (name, email, password) => {
    set({ isLoading: true });

    try {
      // Signup starts a session server-side (rememberMe=false → 1-day
      // cookie), so onboarding and the dashboard continue without a login.
      const data = await api<SignupResponse>("/v1/auth/signup", {
        method: "POST",
        body: JSON.stringify({ name, email, password }),
      });

      set({
        user: data.user,
        session: data.session,
        isLoading: false,
        isAuthenticated: true,
        onboardingCompleted: data.session.onboardingCompleted,
      });
      return { verificationToken: data.verificationToken };
    } catch (err) {
      set({ isLoading: false });
      return { error: toErrorMessage(err) };
    }
  },

  loginWithOAuth: (provider) => {
    // Popup-first OAuth: the backend callback posts the outcome back to
    // this window, then fetchMe() hydrates the session from the new cookie.
    const url = `${env.apiBase}/v1/auth/oauth/${provider}?popup=1`;
    const popup = window.open(url, "notifi-oauth", "width=520,height=640");

    // Popup blocked → full-page redirect; the server sets the cookie either
    // way and fetchMe() restores the session after the bounce back home.
    if (!popup) {
      window.location.assign(`${env.apiBase}/v1/auth/oauth/${provider}`);
      return Promise.resolve(undefined);
    }

    return new Promise<{ error?: string } | undefined>((resolve) => {
      const expectedOrigin = new URL(env.apiBase).origin;

      function onMessage(event: MessageEvent) {
        if (event.origin !== expectedOrigin) return;
        const data = event.data as { type?: string } | null;
        if (data?.type === "oauth:success") {
          cleanup();
          void useAuthStore.getState().fetchMe();
          resolve({});
        } else if (data?.type === "oauth:error") {
          cleanup();
          resolve({
            error: "Sign-in with that provider failed. Please try again.",
          });
        }
      }
      function cleanup() {
        window.removeEventListener("message", onMessage);
      }
      window.addEventListener("message", onMessage);
    });
  },

  logout: async () => {
    // Optimistic local clear; the server clears the cookie regardless.
    set({ user: null, session: null, isAuthenticated: false, onboardingCompleted: false });

    try {
      await api("/v1/auth/logout", { method: "POST" });
    } catch {
      // Already logged out locally; nothing else to do.
    }
  },

  forgotPassword: async (email) => {
    set({ isLoading: true });

    try {
      // Always 200 — never reveals whether the account exists.
      await api("/v1/auth/password/forgot", {
        method: "POST",
        body: JSON.stringify({ email }),
      });
      set({ isLoading: false });
      return {};
    } catch (err) {
      set({ isLoading: false });
      return { error: toErrorMessage(err) };
    }
  },

  resetPassword: async (token, password) => {
    set({ isLoading: true });

    try {
      await api("/v1/auth/password/reset", {
        method: "POST",
        body: JSON.stringify({ token, password }),
      });
      set({ isLoading: false });
      return {};
    } catch (err) {
      set({ isLoading: false });
      return { error: toErrorMessage(err) };
    }
  },

  verifyEmail: async (token) => {
    try {
      await api("/v1/auth/verify-email", {
        method: "POST",
        body: JSON.stringify({ token }),
      });
      return {};
    } catch (err) {
      // Expired tokens answer 410 with "expired" in the detail; pages branch on that word.
      return { error: toErrorMessage(err) };
    }
  },

  resendVerification: async (email) => {
    set({ isLoading: true });

    try {
      // Always 200 regardless of account existence.
      await api("/v1/auth/verify-email/resend", {
        method: "POST",
        body: JSON.stringify({ email }),
      });
      set({ isLoading: false });
      return {};
    } catch (err) {
      set({ isLoading: false });
      return { error: toErrorMessage(err) };
    }
  },

  fetchMe: async () => {
    try {
      const data = await api<MeResponse>("/v1/auth/me");
      set({
        user: data.user,
        isAuthenticated: true,
        onboardingCompleted: data.onboardingCompleted,
      });
    } catch {
      // No valid session cookie (or API down) — stay signed out.
    }
  },

  completeOnboarding: async (input) => {
    try {
      const data = await api<{ status: string; alreadyCompleted?: boolean }>(
        "/v1/auth/onboarding/complete",
        { method: "POST", body: JSON.stringify(input) }
      );
      set({ onboardingCompleted: true });
      return { alreadyCompleted: data.alreadyCompleted };
    } catch (err) {
      return { error: toErrorMessage(err) };
    }
  },
}));
