"use client";

import { useAuthStore } from "@/store/auth-store";

export function useAuth() {
  const {
    user,
    session,
    isLoading,
    isAuthenticated,
    login,
    signup,
    loginWithOAuth,
    logout,
    forgotPassword,
    resetPassword,
    verifyEmail,
  } = useAuthStore();

  return {
    user,
    session,
    isLoading,
    isAuthenticated,
    login,
    signup,
    loginWithOAuth,
    logout,
    forgotPassword,
    resetPassword,
    verifyEmail,
  };
}
