"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { DeviceTokenRegistry } from "./device-token-registry";
import type { Subscriber } from "@/lib/types";

interface SubscriberProfileProps {
  subscriber: Subscriber | null;
  loading: boolean;
}

export function SubscriberProfile({ subscriber, loading }: SubscriberProfileProps) {
  if (loading || !subscriber) {
    return (
      <div className="mx-auto w-full max-w-7xl space-y-6 p-5">
        <Skeleton className="h-8 w-64" />
        <Skeleton className="h-32 w-full rounded-xl" />
        <Skeleton className="h-48 w-full rounded-xl" />
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-7xl space-y-6 p-5">
      <div>
        <h1 className="text-xl font-semibold tracking-tight">{subscriber.name || "Unknown"}</h1>
        <p className="text-sm text-muted-foreground font-mono mt-1">{subscriber.id}</p>
      </div>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <Card className="rounded-xl overflow-hidden">
          <CardHeader className="pb-2">
            <CardTitle className="text-xs font-medium text-muted-foreground">Email</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm">{subscriber.email || "—"}</p>
          </CardContent>
        </Card>
        <Card className="rounded-xl overflow-hidden">
          <CardHeader className="pb-2">
            <CardTitle className="text-xs font-medium text-muted-foreground">Phone</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm">{subscriber.phone || "—"}</p>
          </CardContent>
        </Card>
        <Card className="rounded-xl overflow-hidden">
          <CardHeader className="pb-2">
            <CardTitle className="text-xs font-medium text-muted-foreground">Created</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm">{new Date(subscriber.createdAt).toLocaleDateString()}</p>
          </CardContent>
        </Card>
      </div>

      <Card className="rounded-xl overflow-hidden">
        <CardHeader>
          <CardTitle className="text-base font-semibold">Device Token Registry</CardTitle>
        </CardHeader>
        <CardContent>
          <DeviceTokenRegistry tokens={subscriber.tokens} />
        </CardContent>
      </Card>
    </div>
  );
}
