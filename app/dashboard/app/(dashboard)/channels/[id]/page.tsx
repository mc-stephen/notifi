"use client";

import { use } from "react";
import { useRouter } from "next/navigation";
import { useChannels, useProvidersByChannel } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import { ChannelBadge } from "@/components/custom/channel-badge";
import { HealthIndicator } from "@/components/custom/health-indicator";
import { CodeBlock } from "@/components/custom/code-block";
import { CHANNEL_LABELS } from "@/lib/constants";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import {
  ArrowLeft,
  Settings,
  Shield,
  Zap,
  RotateCcw,
  Send,
} from "lucide-react";

function ChannelDetail({ id }: { id: string }) {
  const channels = useChannels();
  const providers = useProvidersByChannel(id);
  const router = useRouter();

  const channel = channels.find((c) => c.id === id);

  if (!channel) {
    return (
      <div className="flex flex-col items-center justify-center py-20">
        <h2 className="text-lg font-medium">Channel not found</h2>
        <p className="text-sm text-muted-foreground mt-1">The channel {id} does not exist.</p>
        <Button variant="outline" className="mt-4" onClick={() => router.push("/channels")}>
          <ArrowLeft className="size-3.5 mr-1" /> Back to channels
        </Button>
      </div>
    );
  }

  const bestProvider = providers.sort((a, b) => a.priority - b.priority)[0];
  const channelName = CHANNEL_LABELS[channel.type];

  return (
    <div className="space-y-6">
      <PageHeader
        title={channelName}
        description={`Channel configuration and provider management`}
        breadcrumbs={[
          { label: "Dashboard", href: "/" },
          { label: "Channels", href: "/channels" },
          { label: channelName },
        ]}
        actions={
          <div className="flex items-center gap-2">
            <Button size="sm" variant="outline" className="gap-1.5">
              <Send className="size-3.5" /> Test payload
            </Button>
            <Button size="sm" className="gap-1.5">
              <Settings className="size-3.5" /> Configure
            </Button>
          </div>
        }
      />

      <Tabs defaultValue="overview">
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="providers">Providers ({providers.length})</TabsTrigger>
          <TabsTrigger value="config">Configuration</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="mt-4">
          <div className="grid gap-6 lg:grid-cols-3">
            <Card className="lg:col-span-2">
              <CardHeader>
                <CardTitle>Channel Details</CardTitle>
              </CardHeader>
              <CardContent>
                <dl className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <dt className="text-muted-foreground">Channel</dt>
                    <dd className="mt-1"><ChannelBadge channel={channel.type} showIcon /></dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Status</dt>
                    <dd className="mt-1">
                      <Badge variant={channel.enabled ? "default" : "secondary"} className={channel.enabled ? "bg-success/15 text-success border-success/20" : ""}>
                        {channel.enabled ? "Enabled" : "Disabled"}
                      </Badge>
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Providers</dt>
                    <dd className="mt-1 text-sm">{providers.length} configured</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Channel ID</dt>
                    <dd className="mt-1 font-mono text-xs">{channel.id}</dd>
                  </div>
                  <div className="col-span-2">
                    <dt className="text-muted-foreground">Configuration</dt>
                    <dd className="mt-1">
                      <CodeBlock code={JSON.stringify(channel.config, null, 2)} language="JSON" />
                    </dd>
                  </div>
                </dl>
              </CardContent>
            </Card>

            <div className="space-y-6">
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <Zap className="size-4" /> Primary Provider
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-4">
                  {bestProvider ? (
                    <>
                      <div className="text-lg font-medium">{bestProvider.name}</div>
                      <div className="space-y-2">
                        <div className="flex items-center justify-between text-sm">
                          <span className="text-muted-foreground">Health</span>
                          <HealthIndicator status={bestProvider.health} />
                        </div>
                        <div className="flex items-center justify-between text-sm">
                          <span className="text-muted-foreground">Latency</span>
                          <span>{bestProvider.latencyMs}ms</span>
                        </div>
                        <div className="flex items-center justify-between text-sm">
                          <span className="text-muted-foreground">Success Rate</span>
                          <span>{bestProvider.successRate}%</span>
                        </div>
                        <div className="flex items-center justify-between text-sm">
                          <span className="text-muted-foreground">Quota</span>
                          <span>{bestProvider.quotaUsed.toLocaleString()} / {bestProvider.quotaLimit.toLocaleString()}</span>
                        </div>
                        <div className="flex items-center justify-between text-sm">
                          <span className="text-muted-foreground">Region</span>
                          <span>{bestProvider.region}</span>
                        </div>
                      </div>
                    </>
                  ) : (
                    <p className="text-sm text-muted-foreground">No providers configured</p>
                  )}
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <Shield className="size-4" /> Fallback
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  {bestProvider?.fallbackId ? (
                    <p className="text-sm">Fallback to provider: <span className="font-mono text-xs">{bestProvider.fallbackId}</span></p>
                  ) : (
                    <p className="text-sm text-muted-foreground">No fallback configured</p>
                  )}
                </CardContent>
              </Card>
            </div>
          </div>
        </TabsContent>

        <TabsContent value="providers" className="mt-4">
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle>Provider Priority</CardTitle>
                <Button size="sm" variant="outline" className="gap-1.5">
                  <RotateCcw className="size-3.5" /> Reorder
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[60px]">Priority</TableHead>
                    <TableHead>Name</TableHead>
                    <TableHead>Health</TableHead>
                    <TableHead>Latency</TableHead>
                    <TableHead>Success Rate</TableHead>
                    <TableHead>Quota</TableHead>
                    <TableHead>Region</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {providers.sort((a, b) => a.priority - b.priority).map((provider) => (
                    <TableRow key={provider.id}>
                      <TableCell>
                        <Badge variant="secondary" className="text-xs font-mono">{provider.priority}</Badge>
                      </TableCell>
                      <TableCell>
                        <div className="text-sm font-medium">{provider.name}</div>
                      </TableCell>
                      <TableCell>
                        <HealthIndicator status={provider.health} />
                      </TableCell>
                      <TableCell className="text-sm">{provider.latencyMs}ms</TableCell>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <div className="w-16 h-1.5 bg-muted rounded-full overflow-hidden">
                            <div
                              className={`h-full rounded-full ${
                                provider.successRate >= 99 ? "bg-success" :
                                provider.successRate >= 97 ? "bg-warning" : "bg-destructive"
                              }`}
                              style={{ width: `${provider.successRate}%` }}
                            />
                          </div>
                          <span className="text-xs">{provider.successRate}%</span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="text-xs">
                          {provider.quotaUsed.toLocaleString()} / {provider.quotaLimit.toLocaleString()}
                        </div>
                      </TableCell>
                      <TableCell className="text-xs text-muted-foreground">{provider.region}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="config" className="mt-4">
          <Card>
            <CardHeader>
              <CardTitle>Raw Configuration</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">Configuration JSON</label>
                <CodeBlock code={JSON.stringify(channel.config, null, 2)} language="JSON" />
              </div>
              <Separator />
              <div className="space-y-2">
                <label className="text-sm font-medium">Test Payload</label>
                <CodeBlock
                  code={JSON.stringify({
                    channel: channel.type,
                    recipient_id: "rcp_0001",
                    subject: "Test notification",
                    body: "This is a test notification from the Notifi dashboard.",
                    priority: "normal",
                  }, null, 2)}
                  language="JSON"
                />
              </div>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}

export default function ChannelDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  return <ChannelDetail id={id} />;
}
