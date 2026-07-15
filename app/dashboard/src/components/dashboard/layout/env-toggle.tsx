"use client";

import { useEnvironmentStore } from "@/store/environment-store";
import { cn } from "@/lib/utils";
import { ArrowLeftRight } from "lucide-react";

export function EnvToggle() {
  const { environment, toggleEnvironment } = useEnvironmentStore();
  const isProd = environment === "production";

  return (
    <button
      type="button"
      onClick={toggleEnvironment}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-3 py-1 text-xs font-medium transition-all",
        isProd
          ? "border-red-500/25 bg-red-500/8 text-red-400 hover:bg-red-500/15"
          : "border-emerald-500/25 bg-emerald-500/8 text-emerald-400 hover:bg-emerald-500/15",
      )}
    >
      <span
        className={cn(
          "h-1.5 w-1.5 rounded-full",
          isProd ? "bg-red-500" : "bg-emerald-500",
        )}
      />
      {isProd ? "Production" : "Development"}
      <ArrowLeftRight className="h-3 w-3 opacity-50" />
    </button>
  );
}
