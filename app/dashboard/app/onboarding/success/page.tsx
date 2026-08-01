"use client";

import { useRouter } from "next/navigation";
import { PartyPopper, ArrowRight, BookOpen, Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { env } from "@/lib/env";
import { useOnboardingStore } from "@/store/onboarding-store";

export default function SuccessPage() {
  const router = useRouter();
  const { projectName, apiKey, completeOnboarding } = useOnboardingStore();

  const handleGoToDashboard = () => {
    completeOnboarding();
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
        Your Notifi workspace <span className="font-medium text-foreground">{projectName || "project"}</span> is
        ready. Start sending notifications right away.
      </p>

      <div className="mb-10 w-full max-w-sm space-y-4">
        {/* Quick stats */}
        <div className="grid grid-cols-3 gap-3">
          <div className="rounded-xl border p-3">
            <div className="text-2xl font-bold text-primary">3</div>
            <div className="text-xs text-muted-foreground">Channels</div>
          </div>
          <div className="rounded-xl border p-3">
            <div className="text-2xl font-bold text-primary">1</div>
            <div className="text-xs text-muted-foreground">API Key</div>
          </div>
          <div className="rounded-xl border p-3">
            <div className="text-2xl font-bold text-primary">0</div>
            <div className="text-xs text-muted-foreground">Sent</div>
          </div>
        </div>

        {/* API Key reminder */}
        {apiKey && (
          <div className="rounded-xl border bg-muted/30 p-4 text-left">
            <div className="mb-2 text-xs font-medium text-muted-foreground">
              Your API Key (copy it now — you won&apos;t see it again)
            </div>
            <div className="flex items-center gap-2">
              <code className="flex-1 truncate font-mono text-xs">
                {apiKey.substring(0, 12)}••••••••••••
              </code>
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2"
                onClick={() => navigator.clipboard.writeText(apiKey)}
              >
                <Copy className="size-3" />
              </Button>
            </div>
          </div>
        )}
      </div>

      <div className="flex w-full max-w-sm flex-col gap-3">
        <Button onClick={handleGoToDashboard} className="w-full">
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
