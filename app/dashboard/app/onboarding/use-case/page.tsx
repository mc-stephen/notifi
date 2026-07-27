"use client";

import { useState } from "react";
import {
  Building2,
  ShoppingBag,
  Mail,
  MessageSquare,
  Globe,
  Layers,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { OnboardingNav } from "@/components/custom/onboarding/onboarding-nav";
import { useOnboardingStore } from "@/store/onboarding-store";

const USE_CASES = [
  {
    id: "transactional",
    label: "Transactional",
    description: "Order confirmations, password resets, account alerts",
    icon: Mail,
  },
  {
    id: "marketing",
    label: "Marketing",
    description: "Promotions, newsletters, product announcements",
    icon: ShoppingBag,
  },
  {
    id: "collaboration",
    label: "Collaboration",
    description: "Team updates, mentions, task assignments",
    icon: MessageSquare,
  },
  {
    id: "alerts",
    label: "System Alerts",
    description: "Infrastructure monitoring, incident notifications",
    icon: Layers,
  },
  {
    id: "marketplace",
    label: "Marketplace",
    description: "Order updates, delivery tracking, seller notifications",
    icon: Globe,
  },
  {
    id: "enterprise",
    label: "Enterprise",
    description: "Internal comms, compliance, multi-department workflows",
    icon: Building2,
  },
];

export default function UseCasePage() {
  const { useCase, updateData, nextStep, getStepRoute } = useOnboardingStore();
  const [selected, setSelected] = useState<string | null>(useCase);

  const handleContinue = () => {
    if (selected) {
      updateData({ useCase: selected });
      nextStep();
      // Navigate is handled by layout sync, but we push manually too
      window.location.href = getStepRoute(
        useOnboardingStore.getState().currentStep
      );
    }
  };

  return (
    <div className="flex flex-col">
      <h1 className="mb-2 text-2xl font-bold tracking-tight">
        What will you use Notifi for?
      </h1>
      <p className="mb-8 text-muted-foreground">
        This helps us tailor your experience. You can always change this later.
      </p>

      <div className="mb-10 grid gap-3 sm:grid-cols-2">
        {USE_CASES.map((uc) => {
          const Icon = uc.icon;
          return (
            <button
              key={uc.id}
              onClick={() => setSelected(uc.id)}
              className={cn(
                "flex flex-col items-start gap-2 rounded-xl border p-4 text-left transition-colors hover:bg-muted/50",
                selected === uc.id &&
                  "border-primary bg-primary/5 ring-1 ring-primary"
              )}
            >
              <div
                className={cn(
                  "flex size-9 items-center justify-center rounded-lg",
                  selected === uc.id
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted text-muted-foreground"
                )}
              >
                <Icon className="size-4" />
              </div>
              <div>
                <div className="font-medium">{uc.label}</div>
                <div className="text-xs text-muted-foreground">
                  {uc.description}
                </div>
              </div>
            </button>
          );
        })}
      </div>

      <OnboardingNav
        showSkip
        onNext={handleContinue}
        nextLabel="Continue"
      />
    </div>
  );
}
