"use client";

import { FlaskConical } from "lucide-react";
import { useEnvironmentStore } from "@/store/environment-store";

export function EnvironmentBanner() {
  const currentEnvironment = useEnvironmentStore((s) => s.currentEnvironment);
  const hydrated = useEnvironmentStore((s) => s.hydrated);

  if (!hydrated || currentEnvironment !== "development") return null;

  return (
    <div className="flex items-center gap-3 border-b border-warning/30 bg-warning/10 px-4 py-2.5 lg:px-6">
      <FlaskConical className="size-4 shrink-0 text-amber-700 dark:text-amber-400" />
      <p className="text-sm font-medium text-amber-700 dark:text-amber-400">
        You&apos;re in Development — notifications are simulated, no real deliveries.
      </p>
    </div>
  );
}
