"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { PartyPopper, ArrowRight, BookOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { env } from "@/lib/env";
import { useOnboardingStore } from "@/store/onboarding-store";
import { useAuth } from "@/hooks/use-auth";

export default function SuccessPage() {
  const router = useRouter();
  const {
    orgName,
    orgLogo,
    projectName,
    projectDescription,
    selectedChannels,
    completeOnboarding: markComplete,
  } = useOnboardingStore();
  const { completeOnboarding } = useAuth();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Persisting the first org + project is what flips the server-side
  // onboarding flag that unlocks the dashboard.
  const handleGoToDashboard = async () => {
    setError(null);
    setSaving(true);

    // New projects always start in development; staging/production come later.
    const result = await completeOnboarding({
      organization: {
        name: orgName.trim() || "My Organization",
        logoUrl: orgLogo,
      },
      project: {
        name: projectName.trim() || "My App",
        description: projectDescription || null,
        environment: "development",
      },
    });

    setSaving(false);
    if (result?.error) {
      setError(result.error);
      return;
    }

    markComplete();
    router.push("/");
  };

  return (
    <div className="flex flex-col items-center text-center">
      <div className="mb-6 flex size-16 items-center justify-center rounded-2xl bg-green-500/10 text-green-600 dark:text-green-400">
        <PartyPopper className="size-8" />
      </div>

      <h1 className="mb-2 text-3xl font-bold tracking-tight">
        You&apos;re all set!
      </h1>
      <p className="mb-8 max-w-md text-lg text-muted-foreground">
        Your Notifi workspace{" "}
        <span className="font-medium text-foreground">
          {projectName || "project"}
        </span>{" "}
        is ready. Start sending notifications right away.
      </p>

      {error && (
        <div className="mb-6 w-full max-w-sm text-left">
          <ErrorBanner message={error} />
        </div>
      )}

      <div className="mb-10 w-full max-w-sm">
        {/* Quick stats */}
        <div className="grid grid-cols-2 gap-3">
          <div className="rounded-xl border p-3">
            <div className="text-2xl font-bold text-primary">
              {selectedChannels.length}
            </div>
            <div className="text-xs text-muted-foreground">Channels</div>
          </div>
          <div className="rounded-xl border p-3">
            <div className="text-2xl font-bold text-primary">0</div>
            <div className="text-xs text-muted-foreground">Sent</div>
          </div>
        </div>
      </div>

      <div className="flex w-full max-w-sm flex-col gap-3">
        <Button
          onClick={handleGoToDashboard}
          disabled={saving}
          className="w-full"
        >
          Go to Dashboard
          <ArrowRight className="ml-2 size-4" />
        </Button>
        <Button
          variant="outline"
          onClick={() => window.open(env.docs(), "_blank")}
          className="w-full"
        >
          <BookOpen className="mr-2 size-4" />
          Read the Docs
        </Button>
      </div>
    </div>
  );
}
