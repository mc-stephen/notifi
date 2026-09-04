"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { Toaster } from "@/components/ui/sonner";

import { Sidebar } from "@/components/layout/sidebar";
import { Topbar } from "@/components/layout/topbar";
import { CommandPalette } from "@/components/layout/command-palette";
import { VerifyEmailBanner } from "@/components/custom/dashboard/verify-email-banner";
import { EnvironmentBanner } from "@/components/custom/dashboard/environment-banner";
import { useAuthStore } from "@/store/auth-store";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const router = useRouter();
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const onboardingCompleted = useAuthStore((s) => s.onboardingCompleted);
  const authHydrated = useAuthStore((s) => s.authHydrated);

  // Onboarding is mandatory: accounts without an org + project (the
  // server-derived flag) are sent back into the flow on every visit.
  useEffect(() => {
    if (isAuthenticated && !onboardingCompleted) {
      router.replace("/onboarding/welcome");
    }
  }, [isAuthenticated, onboardingCompleted, router]);

  // Until fetchMe settles we don't know whether the session is valid — hold
  // the content so neither the dashboard nor the onboarding redirect flash.
  if (!authHydrated) {
    return (
      <div className="flex h-screen overflow-hidden">
        <div className="hidden lg:block h-full">
          <Sidebar />
        </div>
        <div className="flex flex-1 flex-col overflow-hidden">
          <Topbar />
          <main className="flex-1 overflow-y-auto bg-muted/40" />
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen overflow-hidden">
      {/* Desktop sidebar */}
      <div className="hidden lg:block h-full">
        <Sidebar />
      </div>

      {/* Main area */}
      <div className="flex flex-1 flex-col overflow-hidden">
        <Topbar />
        <EnvironmentBanner />
        <VerifyEmailBanner />
        <main className="flex-1 min-h-0 overflow-y-auto bg-muted/40">
          <div className="mx-auto flex min-h-full max-w-[1400px] flex-col p-6 lg:p-8">
            {children}
          </div>
        </main>
      </div>

      {/* Command palette */}
      <CommandPalette />
      <Toaster position="top-right" />
    </div>
  );
}
