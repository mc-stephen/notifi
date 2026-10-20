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

type LoginResponse = { user: User; session: Session } | { requiresTotp: true };
type SignupResponse = { user: User; session: Session } & { verificationToken?: string };
type MeResponse = { user: User; onboardingCompleted: boolean };
type TotpSetupResponse = { totpSecret: string; totpUri: string };

/** Payload for `POST /app/auth/onboarding/complete`. */
export type CompleteOnboardingInput = {
  project: {
    name: string;
    description?: string | null;
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
  /**
   * True once the session question is settled (fetchMe returned or failed).
   * Before that, the store doesn't know whether the cookie is valid — the
   * dashboard gate renders nothing to avoid unauthenticated flashes.
   */
  authHydrated: boolean;
  /** Server-derived: owns ≥1 org with ≥1 project (can skip onboarding). */
  onboardingCompleted: boolean;

  login: (
    email: string,
    password: string,
    rememberMe: boolean,
    totpCode?: string
  ) => Promise<{ error?: string; requiresTotp?: boolean }>;
  signup: (
    name: string,
    email: string,
    password: string
  ) => Promise<{ error?: string; verificationToken?: string }>;
  loginWithOAuth: (
    provider: OAuthProvider
  ) => Promise<{ error?: string; requiresTotp?: boolean } | undefined>;
  logout: () => Promise<void>;
  totpChallenge: (email: string, code: string) => Promise<{ error?: string }>;
  totpSetup: () => Promise<TotpSetupResponse | { error: string }>;
  totpVerify: (code: string) => Promise<{ error?: string }>;
  totpDisable: (password: string | null, code: string) => Promise<{ error?: string }>;
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
  authHydrated: false,
  onboardingCompleted: false,

  login: async (email, password, rememberMe, totpCode) => {
    set({ isLoading: true });

    try {
      const data = await api<LoginResponse>("/app/auth/login", {
        method: "POST",
        body: JSON.stringify(
          totpCode ? { email, password, rememberMe, totpCode } : { email, password, rememberMe }
        ),
      });

      // Second step required: no session yet — the caller shows the OTP
      // step, which completes via totpChallenge (no password resend).
      if ("requiresTotp" in data) {
        set({ isLoading: false });
        return { requiresTotp: true as const };
      }

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

  totpChallenge: async (email, code) => {
    set({ isLoading: true });

    try {
      const data = await api<{ user: User; session: Session }>("/app/auth/totp/challenge", {
        method: "POST",
        body: JSON.stringify({ email, code }),
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

  totpSetup: async () => {
    try {
      return await api<TotpSetupResponse>("/app/auth/totp/setup", { method: "POST" });
    } catch (err) {
      return { error: toErrorMessage(err) };
    }
  },

  totpVerify: async (code) => {
    try {
      await api<{ status: string }>("/app/auth/totp/verify", {
        method: "POST",
        body: JSON.stringify({ code }),
      });
      await useAuthStore.getState().fetchMe();
      return {};
    } catch (err) {
      return { error: toErrorMessage(err) };
    }
  },

  totpDisable: async (password, code) => {
    try {
      await api<{ status: string }>("/app/auth/totp/disable", {
        method: "POST",
        body: JSON.stringify(password ? { password, code } : { code }),
      });
      await useAuthStore.getState().fetchMe();
      return {};
    } catch (err) {
      return { error: toErrorMessage(err) };
    }
  },

  signup: async (name, email, password) => {
    set({ isLoading: true });

    try {
      // Signup starts a session server-side (rememberMe=false → 1-day
      // cookie), so onboarding and the dashboard continue without a login.
      const data = await api<SignupResponse>("/app/auth/signup", {
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
    const url = `${env.apiBase}/app/auth/oauth/${provider}?popup=1`;

    // Loading state drives the Sign in/Sign up buttons (disabled + spinner)
    // for the whole attempt — set it before the popup opens.
    set({ isLoading: true });

    const popup = window.open(url, "notifi-oauth", "width=520,height=640");

    // Popup blocked → full-page redirect; the server sets the cookie either
    // way and fetchMe() restores the session after the bounce back home.
    if (!popup) {
      window.location.assign(`${env.apiBase}/app/auth/oauth/${provider}`);
      return Promise.resolve(undefined);
    }

    return new Promise<{ error?: string; requiresTotp?: boolean } | undefined>((resolve) => {
      const expectedOrigin = new URL(env.apiBase).origin;
      let settled = false;

      function onMessage(event: MessageEvent) {
        if (settled || event.origin !== expectedOrigin) return;
        const data = event.data as { type?: string } | null;
        if (data?.type === "oauth:success") {          settled = true;
          cleanup();
          // Await hydration before resolving: callers navigate right after,
          // and the onboarding gate must already know both flags. Loading
          // stays on until the store is hydrated.
          void (async () => {
            await useAuthStore.getState().fetchMe();
            set({ isLoading: false });
            resolve({});
          })();
        } else if (data?.type === "oauth:totp_required") {
          // TOTP-enabled account: finish on the login page's OTP step.
          settled = true;
          cleanup();
          set({ isLoading: false });
          resolve({ requiresTotp: true });
        } else if (data?.type === "oauth:error") {
          settled = true;
          cleanup();
          set({ isLoading: false });
          resolve({
            error: "Sign-in with that provider failed. Please try again.",
          });
        }
      }
      function cleanup() {
        window.removeEventListener("message", onMessage);
        clearInterval(closePoll);
      }

      // Abandonment: the user closes the popup without completing the flow.
      // `popup.closed` stays readable cross-origin; without this the loading
      // state would be stuck forever.
      const closePoll = window.setInterval(() => {
        if (!popup.closed) return;
        settled = true;
        cleanup();
        set({ isLoading: false });
        resolve(undefined);
      }, 500);

      window.addEventListener("message", onMessage);
    });
  },

  logout: async () => {
    // Optimistic local clear; the server clears the cookie regardless.
    set({ user: null, session: null, isAuthenticated: false, onboardingCompleted: false });

    try {
      await api("/app/auth/logout", { method: "POST" });
    } catch {
      // Already logged out locally; nothing else to do.
    }
  },

  forgotPassword: async (email) => {
    set({ isLoading: true });

    try {
      // Always 200 — never reveals whether the account exists.
      await api("/app/auth/password/forgot", {
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
      await api("/app/auth/password/reset", {
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
      await api("/app/auth/verify-email", {
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
      await api("/app/auth/verify-email/resend", {
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
      const data = await api<MeResponse>("/app/auth/me");
      set({
        user: data.user,
        isAuthenticated: true,
        onboardingCompleted: data.onboardingCompleted,
        authHydrated: true,
      });
    } catch {
      // No valid session cookie (or API down) — stay signed out, but the
      // session question is now settled.
      set({ authHydrated: true });
    }
  },

  completeOnboarding: async (input) => {
    try {
      const data = await api<{ status: string; alreadyCompleted?: boolean }>(
        "/app/auth/onboarding/complete",
        { method: "POST", body: JSON.stringify(input) }
      );
      set({ onboardingCompleted: true });
      return { alreadyCompleted: data.alreadyCompleted };
    } catch (err) {
      return { error: toErrorMessage(err) };
    }
  },
}));

/**
 * Landing route after any successful sign-in — decided by the server-derived
 * onboarding flag so users land directly where they belong (no dashboard
 * bounce).
 */
export function postAuthDestination(): string {
  return useAuthStore.getState().onboardingCompleted
    ? "/"
    : "/onboarding/welcome";
}
