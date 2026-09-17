"use client";

import { useRouter } from "next/navigation";
import { useChannels, useProviders } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import { CHANNEL_ICON_MAP } from "@/components/custom/channel-badge";
import { HealthIndicator } from "@/components/custom/health-indicator";
import { CHANNEL_LABELS } from "@/lib/constants";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { StatCard } from "@/components/custom/stat-card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  Radio,
  Settings,
  Zap,
  TrendingUp,
  AlertTriangle,
  Shield,
} from "lucide-react";

export default function ChannelsPage() {
  const router = useRouter();
  const channels = useChannels();
  const providers = useProviders();

  const enabledCount = channels.filter((c) => c.enabled).length;
  const healthyProviders = providers.filter(
    (p) => p.connected && p.health === "healthy",
  ).length;
  const connectedProviders = providers.filter((p) => p.connected);
  const avgLatency =
    connectedProviders.length > 0
      ? Math.round(
          connectedProviders.reduce((acc, p) => acc + p.latencyMs, 0) /
            connectedProviders.length,
        )
      : 0;
  const avgSuccessRate =
    connectedProviders.length > 0
      ? (
          connectedProviders.reduce((acc, p) => acc + p.successRate, 0) /
          connectedProviders.length
        ).toFixed(1)
      : "0";

  return (
    <div className="space-y-6">
      <PageHeader
        title="Channels"
        description={`${enabledCount} of ${channels.length} channels enabled`}
      />

      {/* Summary cards */}
      <div className="grid gap-4 md:grid-cols-4">
        <StatCard label="Active Channels" value={enabledCount} sub={`of ${channels.length} configured`} icon={Radio} />
        <StatCard label="Healthy Providers" value={`${healthyProviders}/${connectedProviders.length}`} sub="all systems operational" icon={Zap} />
        <StatCard label="Avg Latency" value={`${avgLatency}ms`} sub="within target" icon={TrendingUp} />
        <StatCard label="Success Rate" value={`${avgSuccessRate}%`} sub="above 95% threshold" icon={AlertTriangle} />
      </div>

      {/* Channel cards */}
      <div className="grid gap-4 md:grid-cols-2">
        {channels.map((channel) => {
          const channelProviders = channel.providerIds
            .map((id) => providers.find((p) => p.id === id))
            .filter((p): p is NonNullable<typeof p> => !!p);
          const primary = channelProviders[0];
          const fallbacks = channelProviders.slice(1);
          const ChannelIcon = CHANNEL_ICON_MAP[channel.type];

          return (
            <Card
              key={channel.id}
              className="cursor-pointer hover:bg-muted/30 transition-colors"
              onClick={() => router.push(`/channels/${channel.id}`)}
            >
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex items-center gap-3">
                    <div className="flex size-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
                      <ChannelIcon className="size-4" />
                    </div>
                    <div>
                      <CardTitle className="text-sm">
                        {CHANNEL_LABELS[channel.type]}
                      </CardTitle>
                      <div className="flex items-center gap-2 mt-0.5">
                        <Badge
                          variant={channel.enabled ? "default" : "secondary"}
                          className={
                            channel.enabled
                              ? "bg-success/15 text-success border-success/20"
                              : ""
                          }
                        >
                          {channel.enabled ? "Enabled" : "Disabled"}
                        </Badge>
                        {channelProviders.length > 0 && (
                          <span className="text-xs text-muted-foreground">
                            {channelProviders.length} provider
                            {channelProviders.length > 1 ? "s" : ""}
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    aria-label="Channel settings"
                    onClick={(e) => {
                      e.stopPropagation();
                      router.push(`/channels/${channel.id}`);
                    }}
                  >
                    <Settings className="size-3.5" />
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="pt-0">
                {primary ? (
                  <div className="space-y-2">
                    <Separator />
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-muted-foreground">
                        Primary: {primary.name}
                      </span>
                      <HealthIndicator status={primary.health} />
                    </div>
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-muted-foreground">
                        Latency: {primary.latencyMs}ms
                      </span>
                      <span className="text-muted-foreground">
                        Success: {primary.successRate}%
                      </span>
                    </div>
                    {fallbacks.length > 0 && (
                      <div className="flex items-center gap-1.5 text-xs">
                        <Shield className="size-3 text-muted-foreground" />
                        <span className="text-muted-foreground">
                          Fallback:{" "}
                          {fallbacks.map((f) => f.name).join(", ")}
                        </span>
                      </div>
                    )}
                  </div>
                ) : (
                  <p className="text-xs text-muted-foreground">
                    No providers assigned.{" "}
                    <Button
                      variant="link"
                      size="sm"
                      className="h-auto p-0 text-xs"
                      onClick={(e) => {
                        e.stopPropagation();
                        router.push(`/channels/${channel.id}`);
                      }}
                    >
                      Add one →
                    </Button>
                  </p>
                )}
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
