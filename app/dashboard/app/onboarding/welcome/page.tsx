"use client";

import { Rocket } from "lucide-react";
import { useAuthStore } from "@/store/auth-store";
import { OnboardingNav } from "@/components/custom/onboarding/onboarding-nav";

export default function WelcomePage() {
  const user = useAuthStore((s) => s.user);

  return (
    <div className="flex flex-col items-center text-center">
      <div className="mb-6 flex size-16 items-center justify-center rounded-2xl bg-primary/10 text-primary">
        <Rocket className="size-8" />
      </div>

      <h1 className="mb-2 text-3xl font-bold tracking-tight">
        Welcome to Notifi{user?.name ? `, ${user.name}` : ""}
      </h1>
      <p className="mb-8 max-w-md text-lg text-muted-foreground">
        Let&apos;s set up your notification platform in just a few steps.
      </p>

      <div className="mb-10 w-full max-w-md space-y-3 text-left">
        {[
          {
            step: "1",
            text: "Tell us about your organization",
          },
          {
            step: "2",
            text: "Create your first project",
          },
          {
            step: "3",
            text: "Configure notification channels",
          },
        ].map((item) => (
          <div
            key={item.step}
            className="flex items-center gap-3 rounded-lg border p-3"
          >
            <div className="flex size-7 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-medium">
              {item.step}
            </div>
            <span className="text-sm">{item.text}</span>
          </div>
        ))}
      </div>

      <div className="w-full max-w-md">
        <OnboardingNav showBack={false} nextLabel="Get Started" />
      </div>
    </div>
  );
}
