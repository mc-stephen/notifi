"use client";

import Link from "next/link";
import { cn } from "@/lib/utils";
import {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
} from "@/components/ui/tooltip";
import { Zap } from "lucide-react";
import { useBilling } from "@/hooks";

function planDescription(notificationsPerMonth: number | null) {
  if (notificationsPerMonth === null) return "Unlimited notifications/mo";
  return `${notificationsPerMonth.toLocaleString()} notifications/mo`;
}

export function PlanBanner({ collapsed = false }: { collapsed?: boolean }) {
  const { subscription, loading, error } = useBilling();
  const plan = subscription?.plan ?? null;

  // Never paint a guessed plan: pulse while loading, hide on error.
  if (loading || error || !plan) {
    if (error || !loading) return null;
    if (collapsed) return null;
    return (
      <div
        className="flex items-center justify-between rounded-lg border border-sidebar-border px-2 py-2"
        aria-label="Loading subscription"
      >
        <div className="min-w-0 flex-1 animate-pulse space-y-1.5">
          <div className="h-3 w-2/3 rounded bg-sidebar-accent" />
        </div>
      </div>
    );
  }

  const canUpgrade = plan.priceCents !== null;

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
        "bg-sidebar group flex items-center justify-between rounded-lg border border-sidebar-border px-2 py-2 transition-colors",
        "hover:border-primary/30 hover:bg-sidebar-accent/40",
      )}
    >
      <div className="min-w-0 flex-1">
        <span className="text-xs font-medium text-sidebar-foreground/80">
          {plan.name} <span className="text-sidebar-foreground/50">·</span>{" "}
          <span className="text-sidebar-foreground/50">
            {planDescription(plan.capabilities.notificationsPerMonth)}
          </span>
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
