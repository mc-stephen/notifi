"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

interface ProviderStatus {
  name: string;
  status: "healthy" | "degraded" | "outage";
  latency?: number;
}

interface ProviderStatusProps {
  providers: ProviderStatus[];
  loading?: boolean;
}

const statusConfig = {
  healthy: { dot: "bg-emerald-500", bg: "bg-emerald-500/8 text-emerald-400", label: "Healthy" },
  degraded: { dot: "bg-amber-500", bg: "bg-amber-500/8 text-amber-400", label: "Degraded" },
  outage: { dot: "bg-red-500", bg: "bg-red-500/8 text-red-400", label: "Outage" },
};

export function ProviderStatusBoard({ providers, loading }: ProviderStatusProps) {
  if (loading) {
    return (
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-32" />
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className="h-10 w-full" />
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="rounded-xl overflow-hidden">
      <CardHeader className="pb-3">
        <CardTitle className="text-base font-semibold">Provider Status</CardTitle>
      </CardHeader>
      <CardContent className="p-0">
        <div className="divide-y divide-border">
          {providers.map((p) => {
            const cfg = statusConfig[p.status];
            return (
              <div
                key={p.name}
                className="flex items-center justify-between px-5 py-3 transition-colors hover:bg-secondary/50"
              >
                <div className="flex items-center gap-3">
                  <span className={cn("h-2 w-2 rounded-full", cfg.dot)} />
                  <span className="text-sm font-medium">{p.name}</span>
                </div>
                <div className="flex items-center gap-3">
                  {p.latency && (
                    <span className="text-xs text-muted-foreground">{p.latency}ms</span>
                  )}
                  <span className={cn("rounded-full px-2.5 py-0.5 text-xs font-medium", cfg.bg)}>
                    {cfg.label}
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
