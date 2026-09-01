"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { PartyPopper, ArrowRight, BookOpen, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { links } from "@/lib/env";
import { useOnboardingStore } from "@/store/onboarding-store";
import { useAuth } from "@/hooks/use-auth";

export default function SuccessPage() {
  const router = useRouter();
  const {
    projectName,
    projectDescription,
    completeOnboarding: markComplete,
  } = useOnboardingStore();
  const { completeOnboarding } = useAuth();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Persisting the first project is what flips the server-side onboarding
  // flag that unlocks the dashboard.
  const handleGoToDashboard = async () => {
    setError(null);
    setSaving(true);

    // New projects always start in development mode (the project-level
    // environment gate); staging/production are toggled in the dashboard.
    const result = await completeOnboarding({
      project: {
        name: projectName.trim() || "My App",
        description: projectDescription || null,
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

      <div className="flex w-full max-w-sm flex-col gap-3">
        <Button
          onClick={handleGoToDashboard}
          disabled={saving}
          className="w-full"
        >
          {saving ? (
            <>
              <Loader2 className="mr-2 size-4 animate-spin" />
              Finishing up...
            </>
          ) : (
            <>
              Go to Dashboard
              <ArrowRight className="ml-2 size-4" />
            </>
          )}
        </Button>
        <Button
          variant="outline"
          onClick={() => window.open(links.docs, "_blank")}
          className="w-full"
        >
          <BookOpen className="mr-2 size-4" />
          Read the Docs
        </Button>
      </div>
    </div>
  );
}
