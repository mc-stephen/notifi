"use client";

import { useAuthStore } from "@/store/auth-store";

export function useAuth() {
  const {
    user,
    session,
    isLoading,
    isAuthenticated,
    onboardingCompleted,
    login,
    signup,
    loginWithOAuth,
    logout,
    forgotPassword,
    resetPassword,
    verifyEmail,
    resendVerification,
    fetchMe,
    completeOnboarding,
  } = useAuthStore();

  return {
    user,
    session,
    isLoading,
    isAuthenticated,
    onboardingCompleted,
    login,
    signup,
    loginWithOAuth,
    logout,
    forgotPassword,
    resetPassword,
    verifyEmail,
    resendVerification,
    fetchMe,
    completeOnboarding,
  };
}
