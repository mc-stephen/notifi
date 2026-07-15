"use client";

import { ShieldCheck } from "lucide-react";

export function IdempotencyIndicator() {
  return (
    <div className="flex items-center gap-2 rounded-xl border border-primary/20 bg-primary/5 px-4 py-3 text-xs text-muted-foreground">
      <ShieldCheck className="h-4 w-4 text-primary shrink-0" />
      <span>
        Requests are idempotency-guarded via the{" "}
        <code className="rounded bg-secondary px-1.5 py-0.5 font-mono text-[10px] border border-border/40">
          Idempotency-Key
        </code>{" "}
        header. Duplicate requests within 24h are safely ignored.
      </span>
    </div>
  );
}
