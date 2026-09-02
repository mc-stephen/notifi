"use client";

import Link from "next/link";
import { cn } from "@/lib/utils";
import {
  CURRENT_PLAN,
  PLAN_LABELS,
  PLAN_DESCRIPTIONS,
  PLAN_UPGRADEABLE,
} from "@/lib/constants";
import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/components/ui/tooltip";
import { Zap } from "lucide-react";

export function PlanBanner({ collapsed = false }: { collapsed?: boolean }) {
  const label = PLAN_LABELS[CURRENT_PLAN];
  const description = PLAN_DESCRIPTIONS[CURRENT_PLAN];
  const canUpgrade = PLAN_UPGRADEABLE[CURRENT_PLAN];

  if (collapsed) {
    if (!canUpgrade) return null;

    return (
      <Tooltip>
        <TooltipTrigger
          render={
            <Link
              href="/billing"
              className="flex h-8 w-full items-center justify-center rounded-lg text-sidebar-foreground/50 transition-colors hover:bg-sidebar-accent/60 hover:text-sidebar-foreground"
            />
          }
        >
          <Zap className="size-4" />
        </TooltipTrigger>
        <TooltipContent side="right">Upgrade plan</TooltipContent>
      </Tooltip>
    );
  }

  return (
    <Link
      href="/billing"
      className={cn(
        "bg-sidebar group flex items-center justify-between rounded-lg border border-sidebar-border px-3 py-2 transition-colors",
        "hover:border-primary/30 hover:bg-sidebar-accent/40",
      )}
    >
      <div className="min-w-0 flex-1">
        <span className="text-xs font-medium text-sidebar-foreground/80">
          {label} <span className="text-sidebar-foreground/50">·</span>{" "}
          <span className="text-sidebar-foreground/50">{description}</span>
        </span>
      </div>
      {canUpgrade && (
        <span className="ml-2 shrink-0 rounded-md bg-primary/10 px-1.5 py-0.5 text-[10px] font-semibold text-primary transition-colors group-hover:bg-primary/20">
          Upgrade
        </span>
      )}
    </Link>
  );
}
