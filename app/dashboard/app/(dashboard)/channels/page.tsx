"use client";

import { useRouter } from "next/navigation";
import { useChannels, useProviders } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import { ChannelBadge } from "@/components/custom/channel-badge";
import { HealthIndicator } from "@/components/custom/health-indicator";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  Radio,
  Settings,
  Zap,
  TrendingUp,
  AlertTriangle,
} from "lucide-react";

export default function ChannelsPage() {
  const router = useRouter();
  const channels = useChannels();
  const providers = useProviders();

  const enabledCount = channels.filter((c) => c.enabled).length;
  const healthyProviders = providers.filter((p) => p.health === "healthy").length;
  const avgLatency = Math.round(providers.reduce((acc, p) => acc + p.latencyMs, 0) / providers.length);
  const avgSuccessRate = (providers.reduce((acc, p) => acc + p.successRate, 0) / providers.length).toFixed(1);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Channels & Providers"
        description={`${enabledCount} active channels, ${providers.length} providers`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Channels" }]}
        actions={
          <Button size="sm" className="gap-1.5">
            <Radio className="size-3.5" /> Configure channel
          </Button>
        }
      />

      {/* Summary cards */}
      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Active Channels</span>
              <Radio className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{enabledCount}</div>
            <p className="text-xs text-muted-foreground mt-1">of {channels.length} configured</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Healthy Providers</span>
              <Zap className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{healthyProviders}/{providers.length}</div>
            <p className="text-xs text-muted-foreground mt-1">all systems operational</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Avg Latency</span>
              <TrendingUp className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{avgLatency}ms</div>
            <p className="text-xs text-success mt-1">within target</p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Success Rate</span>
              <AlertTriangle className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{avgSuccessRate}%</div>
            <p className="text-xs text-success mt-1">above 95% threshold</p>
          </CardContent>
        </Card>
      </div>

      {/* Channel cards */}
      <div className="grid gap-4 md:grid-cols-2">
        {channels.map((channel) => {
          const channelProviders = providers.filter((p) => p.channelId === channel.id);
          const bestProvider = channelProviders.sort((a, b) => a.priority - b.priority)[0];

          return (
            <Card
              key={channel.id}
              className="cursor-pointer hover:bg-muted/30 transition-colors"
              onClick={() => router.push(`/channels/${channel.id}`)}
            >
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex items-center gap-3">
                    <ChannelBadge channel={channel.type} showIcon />
                    <div>
                      <CardTitle className="text-sm">{channel.type.replace(/-/g, " ").replace(/\b\w/g, (c) => c.toUpperCase())}</CardTitle>
                      <div className="flex items-center gap-2 mt-0.5">
                        <Badge variant={channel.enabled ? "default" : "secondary"} className={channel.enabled ? "bg-success/15 text-success border-success/20" : ""}>
                          {channel.enabled ? "Enabled" : "Disabled"}
                        </Badge>
                        {channelProviders.length > 0 && (
                          <span className="text-xs text-muted-foreground">
                            {channelProviders.length} provider{channelProviders.length > 1 ? "s" : ""}
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                  <Button variant="ghost" size="icon-xs">
                    <Settings className="size-3.5" />
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="pt-0">
                {bestProvider && (
                  <div className="space-y-2">
                    <Separator />
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-muted-foreground">Primary: {bestProvider.name}</span>
                      <HealthIndicator status={bestProvider.health} />
                    </div>
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-muted-foreground">Latency: {bestProvider.latencyMs}ms</span>
                      <span className="text-muted-foreground">Success: {bestProvider.successRate}%</span>
                    </div>
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-muted-foreground">Quota: {bestProvider.quotaUsed.toLocaleString()} / {bestProvider.quotaLimit.toLocaleString()}</span>
                    </div>
                  </div>
                )}
                {channelProviders.length === 0 && (
                  <p className="text-xs text-muted-foreground">No providers configured</p>
                )}
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
