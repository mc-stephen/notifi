"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { Bell } from "lucide-react";
import { Stepper } from "@/components/custom/onboarding/stepper";
import { useOnboardingStore, STEP_ROUTES } from "@/store/onboarding-store";

const STEP_LABELS = [
  "Welcome",
  "Use Case",
  "Organization",
  "Project",
  "API Key",
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
  const { currentStep, isCompleted } = useOnboardingStore();

  // Redirect to dashboard if onboarding is completed
  useEffect(() => {
    if (isCompleted) {
      router.push("/");
    }
  }, [isCompleted, router]);

  // Sync current step with URL
  useEffect(() => {
    const currentPath = window.location.pathname;
    const stepIndex = STEP_ROUTES.indexOf(
      currentPath as (typeof STEP_ROUTES)[number]
    );
    if (stepIndex !== -1 && stepIndex !== currentStep) {
      useOnboardingStore.getState().setStep(stepIndex);
    }
  }, [currentStep]);

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
      <main className="flex-1 overflow-y-auto px-6 py-8">
        <div className="mx-auto max-w-2xl">{children}</div>
      </main>
    </div>
  );
}
