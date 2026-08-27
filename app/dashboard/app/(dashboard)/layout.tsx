"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

import { Sidebar } from "@/components/layout/sidebar";
import { Topbar } from "@/components/layout/topbar";
import { CommandPalette } from "@/components/layout/command-palette";
import { VerifyEmailBanner } from "@/components/custom/dashboard/verify-email-banner";
import { useAuthStore } from "@/store/auth-store";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const router = useRouter();
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const onboardingCompleted = useAuthStore((s) => s.onboardingCompleted);

  // Onboarding is mandatory: accounts without an org + project (the
  // server-derived flag) are sent back into the flow on every visit.
  useEffect(() => {
    if (isAuthenticated && !onboardingCompleted) {
      router.replace("/onboarding/welcome");
    }
  }, [isAuthenticated, onboardingCompleted, router]);

  return (
    <div className="flex h-screen overflow-hidden">
      {/* Desktop sidebar */}
      <div className="hidden lg:block h-full">
        <Sidebar />
      </div>

      {/* Main area */}
      <div className="flex flex-1 flex-col overflow-hidden">
        <Topbar />
        <VerifyEmailBanner />
        <main className="flex-1 overflow-y-auto">
          <div className="mx-auto max-w-[1400px] p-4 lg:p-6">
            {children}
          </div>
        </main>
      </div>

      {/* Command palette */}
      <CommandPalette />
    </div>
  );
}
