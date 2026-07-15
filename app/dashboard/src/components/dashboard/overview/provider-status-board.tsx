"use client";

import { cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Mail, Flame, Apple, MessageCircle, Globe } from "lucide-react";
import type { Provider, ProviderStatus } from "@/lib/types";

const iconMap: Record<string, React.ElementType> = {
  Mail,
  Flame,
  Apple,
  MessageCircle,
  Globe,
};

const statusConfig: Record<ProviderStatus, { label: string; dot: string; bg: string }> = {
  healthy: {
    label: "Healthy",
    dot: "bg-emerald-500",
    bg: "bg-emerald-500/5",
  },
  degraded: {
    label: "Degraded",
    dot: "bg-amber-500",
    bg: "bg-amber-500/5",
  },
  outage: {
    label: "Outage",
    dot: "bg-red-500",
    bg: "bg-red-500/5",
  },
};

interface ProviderStatusBoardProps {
  providers: Provider[];
  loading?: boolean;
}

export function ProviderStatusBoard({ providers, loading }: ProviderStatusBoardProps) {
  if (loading) {
    return (
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-36" />
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 gap-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <Skeleton key={i} className="h-16 rounded-lg" />
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Provider Status</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-3">
          {providers.map((provider) => {
            const Icon = iconMap[provider.icon];
            const status = statusConfig[provider.status];

            return (
              <div
                key={provider.id}
                className={cn(
                  "flex items-center gap-3 rounded-lg border border-border p-3 transition-colors",
                  status.bg,
                )}
              >
                {Icon && (
                  <div className="flex h-8 w-8 items-center justify-center rounded-md bg-secondary">
                    <Icon className="h-4 w-4 text-muted-foreground" />
                  </div>
                )}
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{provider.name}</p>
                  <div className="flex items-center gap-1.5 mt-0.5">
                    <span className={cn("h-1.5 w-1.5 rounded-full", status.dot)} />
                    <span className="text-xs text-muted-foreground">{status.label}</span>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
