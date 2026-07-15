"use client";

import { cn } from "@/lib/utils";
import type { ExecutionStep } from "@/lib/types";
import { CheckCircle2, XCircle, Clock } from "lucide-react";

interface ExecutionTimelineProps {
  steps: ExecutionStep[];
}

const statusIcon = {
  success: CheckCircle2,
  error: XCircle,
  pending: Clock,
};

const statusColor = {
  success: "text-emerald-500",
  error: "text-red-500",
  pending: "text-muted-foreground",
};

const statusBg = {
  success: "border-emerald-500/30",
  error: "border-red-500/30",
  pending: "border-border",
};

export function ExecutionTimeline({ steps }: ExecutionTimelineProps) {
  return (
    <div className="space-y-0">
      {steps.map((step, i) => {
        const Icon = statusIcon[step.status];
        const isLast = i === steps.length - 1;

        return (
          <div key={step.label} className="relative flex gap-3 pb-4 last:pb-0">
            {/* Connector line */}
            {!isLast && (
              <div className="absolute left-[11px] top-5 bottom-0 w-px bg-border" />
            )}

            {/* Icon */}
            <div className="relative z-10 mt-0.5">
              <Icon className={cn("h-5 w-5", statusColor[step.status])} />
            </div>

            {/* Content */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">{step.label}</span>
                <span className="text-xs text-muted-foreground">
                  +{step.offsetMs}ms
                </span>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
