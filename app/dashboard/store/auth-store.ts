import { create } from "zustand";
import type { User, Session, OAuthProvider } from "@/lib/auth-types";

const MOCK_USERS: (User & { password: string })[] = [
  {
    id: "user_1",
    name: "John Doe",
    email: "john@example.com",
    password: "Password123!",
    emailVerified: true,
    createdAt: "2025-01-10T00:00:00Z",
    lastLoginAt: "2025-07-25T00:00:00Z",
  },
  {
    id: "user_2",
    name: "Jane Smith",
    email: "jane@example.com",
    password: "Password456!",
    emailVerified: false,
    createdAt: "2025-03-15T00:00:00Z",
    lastLoginAt: "2025-07-20T00:00:00Z",
  },
];

function generateToken(): string {
  return `mock_token_${Date.now()}_${Math.random().toString(36).slice(2)}`;
}

function createSession(user: User): Session {
  return {
    user,
    token: generateToken(),
    expiresAt: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(),
  };
}

type AuthState = {
  user: User | null;
  session: Session | null;
  isLoading: boolean;
  isAuthenticated: boolean;

  login: (email: string, password: string) => Promise<{ error?: string }>;
  signup: (
    name: string,
    email: string,
    password: string
  ) => Promise<{ error?: string }>;
  loginWithOAuth: (provider: OAuthProvider) => Promise<void>;
  logout: () => void;
  forgotPassword: (email: string) => Promise<{ error?: string }>;
  resetPassword: (
    token: string,
    password: string
  ) => Promise<{ error?: string }>;
  verifyEmail: (token: string) => Promise<{ error?: string }>;
};

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  session: null,
  isLoading: false,
  isAuthenticated: false,

  login: async (email, password) => {
    set({ isLoading: true });

    // Simulate API delay
    await new Promise((resolve) => setTimeout(resolve, 800));

    const mockUser = MOCK_USERS.find(
      (u) => u.email === email && u.password === password
    );

    if (!mockUser) {
      set({ isLoading: false });
      return { error: "Invalid email or password. Please try again." };
    }

    const { password: _, ...user } = mockUser;
    const session = createSession(user);

    if (typeof window !== "undefined") {
      document.cookie = "session_token=mock_session_token; path=/; max-age=604800";
    }

    set({
      user,
      session,
      isLoading: false,
      isAuthenticated: true,
    });

    return {};
  },

  signup: async (name, email, password) => {
    set({ isLoading: true });

    await new Promise((resolve) => setTimeout(resolve, 800));

    const existingUser = MOCK_USERS.find((u) => u.email === email);

    if (existingUser) {
      set({ isLoading: false });
      return { error: "An account with this email already exists." };
    }

    const newUser: User = {
      id: `user_${Date.now()}`,
      name,
      email,
      emailVerified: false,
      createdAt: new Date().toISOString(),
      lastLoginAt: new Date().toISOString(),
    };

    MOCK_USERS.push({ ...newUser, password });

    const session = createSession(newUser);

    if (typeof window !== "undefined") {
      document.cookie = "session_token=mock_session_token; path=/; max-age=604800";
    }

    set({
      user: newUser,
      session,
      isLoading: false,
      isAuthenticated: true,
    });

    return {};
  },

  loginWithOAuth: async (provider) => {
    set({ isLoading: true });

    await new Promise((resolve) => setTimeout(resolve, 1000));

    const oauthUser: User = {
      id: `user_${Date.now()}`,
      name: provider === "google" ? "Google User" : "GitHub User",
      email: `user@${provider}.com`,
      emailVerified: true,
      createdAt: new Date().toISOString(),
      lastLoginAt: new Date().toISOString(),
    };

    const session = createSession(oauthUser);

    if (typeof window !== "undefined") {
      document.cookie = "session_token=mock_session_token; path=/; max-age=604800";
    }

    set({
      user: oauthUser,
      session,
      isLoading: false,
      isAuthenticated: true,
    });
  },

  logout: () => {
    if (typeof window !== "undefined") {
      document.cookie = "session_token=; path=/; max-age=0";
    }

    set({
      user: null,
      session: null,
      isLoading: false,
      isAuthenticated: false,
    });
  },

  forgotPassword: async (email) => {
    set({ isLoading: true });

    await new Promise((resolve) => setTimeout(resolve, 800));

    const existingUser = MOCK_USERS.find((u) => u.email === email);

    if (!existingUser) {
      set({ isLoading: false });
      return { error: "No account found with this email address." };
    }

    set({ isLoading: false });
    return {};
  },

  resetPassword: async (token, password) => {
    set({ isLoading: true });

    await new Promise((resolve) => setTimeout(resolve, 800));

    if (!token || token.length < 10) {
      set({ isLoading: false });
      return { error: "Invalid or expired reset token." };
    }

    set({ isLoading: false });
    return {};
  },

  verifyEmail: async (token) => {
    set({ isLoading: true });

    await new Promise((resolve) => setTimeout(resolve, 1000));

    if (!token || token.length < 10) {
      set({ isLoading: false });
      return { error: "Invalid or expired verification token." };
    }

    set((state) => ({
      user: state.user ? { ...state.user, emailVerified: true } : null,
      isLoading: false,
    }));

    return {};
  },
}));
