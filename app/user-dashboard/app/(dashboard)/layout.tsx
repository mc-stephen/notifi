"use client";

import { Suspense, useEffect } from "react";
import { usePathname, useRouter } from "next/navigation";
import { Toaster } from "@/components/ui/sonner";

import { Sidebar } from "@/components/layout/sidebar";
import { MobileStrip } from "@/components/layout/sidebar-widgets";
import { CommandPalette } from "@/components/layout/command-palette";
import { NavigationProgress } from "@/components/custom/navigation-progress";
import { VerifyEmailBanner } from "@/components/custom/dashboard/verify-email-banner";
import { EnvironmentBanner } from "@/components/custom/dashboard/environment-banner";
import { useAuthStore } from "@/store/auth-store";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const router = useRouter();
  const pathname = usePathname();
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const onboardingCompleted = useAuthStore((s) => s.onboardingCompleted);
  const authHydrated = useAuthStore((s) => s.authHydrated);
  // Flow builder fills the body edge-to-edge (no padded container).
  const isFlowPage = pathname.startsWith("/flows/");

  // Dead or missing sessions (nuked DB, expiry, revocation elsewhere)
  // land here with a stale cookie: fetchMe settles unauthenticated and
  // the dashboard bounces to login instead of rendering userless.
  useEffect(() => {
    if (authHydrated && !isAuthenticated) {
      router.replace(`/auth/login?next=${encodeURIComponent(pathname)}`);
    }
  }, [authHydrated, isAuthenticated, pathname, router]);

  // Onboarding is mandatory: accounts without an org + project (the
  // server-derived flag) are sent back into the flow on every visit.
  useEffect(() => {
    if (isAuthenticated && !onboardingCompleted) {
      router.replace("/onboarding/welcome");
    }
  }, [isAuthenticated, onboardingCompleted, router]);

  // Until fetchMe settles we don't know whether the session is valid — hold
  // the content so neither the dashboard nor the onboarding redirect flash.
  // Same hold while the logged-out bounce is in flight: no "?" avatar state
  // is ever reachable.
  if (!authHydrated || !isAuthenticated) {
    return (
      <div className="flex h-screen overflow-hidden">
        <div className="hidden lg:block h-full">
          <Sidebar />
        </div>
        <div className="flex flex-1 flex-col overflow-hidden">
          <main className="flex-1 overflow-y-auto bg-popover" />
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
      <div className="relative flex flex-1 flex-col overflow-hidden">
        <MobileStrip />
        <Suspense fallback={null}>
          <NavigationProgress className="absolute top-0 right-0 left-0 z-50" />
        </Suspense>
        <EnvironmentBanner />
        <VerifyEmailBanner />
        <main className="flex-1 min-h-0 overflow-y-auto bg-popover">
          <div
            id="dashboard-panel-host"
            className={
              isFlowPage
                ? "relative mx-auto flex min-h-full w-full max-w-none flex-col"
                : "relative mx-auto flex min-h-full max-w-[1400px] flex-col p-6 lg:p-8"
            }
          >
            {children}
          </div>
        </main>
      </div>

      {/* Command palette */}
      <CommandPalette />
      <Toaster position="bottom-right" />
    </div>
  );
}
