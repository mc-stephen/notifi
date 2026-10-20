"use client";

import { useEffect } from "react";
import { useRouter, usePathname } from "next/navigation";
import { Bell } from "lucide-react";
import { Stepper } from "@/components/custom/onboarding/stepper";
import { useOnboardingStore, STEP_ROUTES } from "@/store/onboarding-store";
import { useAuthStore } from "@/store/auth-store";

const STEP_LABELS = [
  "Welcome",
  "Use Case",
  "Project",
  "Channels",
  "Team",
  "Success",
];

export default function OnboardingLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const router = useRouter();
  const pathname = usePathname();
  const { currentStep, isCompleted } = useOnboardingStore();
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const authHydrated = useAuthStore((s) => s.authHydrated);

  // The flow is only entered with a session behind it: without one
  // (logged out, dead session, API unreachable at middleware time),
  // bounce to login instead of rendering steps for a ghost.
  useEffect(() => {
    if (authHydrated && !isAuthenticated) {
      router.replace(`/auth/login?next=${encodeURIComponent(pathname)}`);
    }
  }, [authHydrated, isAuthenticated, pathname, router]);

  // Redirect to dashboard if onboarding is completed
  useEffect(() => {
    if (isCompleted) {
      router.push("/");
    }
  }, [isCompleted, router]);

  // Sync current step with URL
  useEffect(() => {
    const stepIndex = STEP_ROUTES.indexOf(
      pathname as (typeof STEP_ROUTES)[number]
    );
    if (stepIndex !== -1) {
      useOnboardingStore.getState().setStep(stepIndex);
    }
  }, [pathname]);

  // Hold the steps until the session question settles (and while the
  // logged-out bounce is in flight) — no step paint for ghosts.
  if (!authHydrated || !isAuthenticated) {
    return (
      <div className="flex min-h-screen flex-col items-center justify-center bg-background">
        <div className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <Bell className="size-5" />
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen flex-col bg-background">
      {/* Header */}
      <header className="flex items-center justify-between border-b px-6 py-4">
        <div className="flex items-center gap-2">
          <div className="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
            <Bell className="size-4" />
          </div>
          <span className="text-lg font-bold">Notifi</span>
        </div>
        <div className="text-sm text-muted-foreground">
          Step {currentStep + 1} of {STEP_ROUTES.length}
        </div>
      </header>

      {/* Stepper */}
      <div className="border-b px-6 py-4">
        <div className="mx-auto max-w-2xl">
          <Stepper
            currentStep={currentStep}
            totalSteps={STEP_ROUTES.length}
            labels={STEP_LABELS}
          />
        </div>
      </div>

      {/* Content */}
      <main className="relative flex flex-1 flex-col overflow-y-auto px-6 pt-8 pb-8">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 [background-image:radial-gradient(circle,var(--border)_1px,transparent_1px)] [background-size:24px_24px] [mask-image:radial-gradient(ellipse_at_center,transparent_10%,black_70%)] opacity-60"
        />
        <div className="relative mx-auto my-auto w-full max-w-2xl">{children}</div>
      </main>
    </div>
  );
}
