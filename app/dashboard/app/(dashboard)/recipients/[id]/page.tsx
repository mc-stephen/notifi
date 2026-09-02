"use client";

import { use } from "react";
import { useRouter } from "next/navigation";
import { useRecipient, useRecipientNotifications } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import { StatusBadge } from "@/components/custom/status-badge";
import { ChannelBadge } from "@/components/custom/channel-badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/separator";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { format } from "date-fns";
import {
  ArrowLeft,
  Smartphone,
  Monitor,
  Globe,
  Trash2,
  ExternalLink,
} from "lucide-react";
import Link from "next/link";

const PLATFORM_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  android: Smartphone,
  ios: Smartphone,
  macos: Monitor,
  linux: Monitor,
  windows: Monitor,
  browser: Globe,
};

function RecipientOverview({ recipient }: { recipient: NonNullable<ReturnType<typeof useRecipient>> }) {
  return (
    <div className="grid gap-6 lg:grid-cols-3">
      <Card className="lg:col-span-2">
        <CardHeader>
          <CardTitle>Profile</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <dt className="text-muted-foreground">Name</dt>
              <dd className="mt-1 font-medium">{recipient.name}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Email</dt>
              <dd className="mt-1">{recipient.email ?? "—"}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Phone</dt>
              <dd className="mt-1">{recipient.phone ?? "—"}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Language</dt>
              <dd className="mt-1 uppercase">{recipient.language ?? "—"}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Timezone</dt>
              <dd className="mt-1">{recipient.timezone ?? "—"}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Created</dt>
              <dd className="mt-1">{format(new Date(recipient.createdAt), "MMM d, yyyy")}</dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Last Active</dt>
              <dd className="mt-1">
                {recipient.lastActiveAt
                  ? format(new Date(recipient.lastActiveAt), "MMM d, yyyy HH:mm")
                  : "—"}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">Devices</dt>
              <dd className="mt-1">{recipient.devices?.length ?? 0}</dd>
            </div>
          </dl>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Attributes</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {Object.entries(recipient.attributes ?? {}).map(([key, value]) => (
            <div key={key} className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground capitalize">{key}</span>
              <span className="text-sm font-medium">{value}</span>
            </div>
          ))}
          {Object.keys(recipient.attributes ?? {}).length === 0 && (
            <p className="text-sm text-muted-foreground">No attributes set.</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function RecipientDevices({ recipient }: { recipient: NonNullable<ReturnType<typeof useRecipient>> }) {
  const devices = recipient.devices ?? [];

  if (devices.length === 0) {
    return (
      <Card>
        <CardContent className="py-12 text-center text-muted-foreground">
          No devices registered.
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      {devices.map((device) => {
        const PlatformIcon = PLATFORM_ICONS[device.platform] ?? Smartphone;
        return (
          <Card key={device.id}>
            <CardContent className="pt-5 space-y-3">
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-2">
                  <PlatformIcon className="size-4 text-muted-foreground" />
                  <div>
                    <div className="text-sm font-medium capitalize">{device.platform}</div>
                    <div className="text-xs text-muted-foreground">{device.provider}</div>
                  </div>
                </div>
                <Badge
                  variant={device.status === "active" ? "default" : "secondary"}
                  className={
                    device.status === "active"
                      ? "bg-success/15 text-success border-success/20"
                      : device.status === "expired"
                        ? "bg-destructive/15 text-destructive border-destructive/20"
                        : ""
                  }
                >
                  {device.status}
                </Badge>
              </div>
              <Separator />
              <dl className="space-y-2 text-xs">
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">App Version</dt>
                  <dd>{device.appVersion ?? "—"}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Platform Version</dt>
                  <dd>{device.platformVersion ?? "—"}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Last Active</dt>
                  <dd>
                    {device.lastActiveAt
                      ? format(new Date(device.lastActiveAt), "MMM d, HH:mm")
                      : "—"}
                  </dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Token</dt>
                  <dd className="font-mono truncate max-w-[120px]">{device.token}</dd>
                </div>
              </dl>
            </CardContent>
          </Card>
        );
      })}
    </div>
  );
}

function RecipientPreferences({ recipient }: { recipient: NonNullable<ReturnType<typeof useRecipient>> }) {
  const hash = recipient.id.split("").reduce((acc, c) => acc + c.charCodeAt(0), 0);
  return (
    <div className="grid gap-6 lg:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Notification Preferences</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {(["email", "sms", "push-ios", "push-android", "web-push", "slack"] as const).map((channel, i) => {
            const enabled = ((hash + i * 7) % 10) > 2;
            return (
              <div key={channel} className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <ChannelBadge channel={channel} showIcon />
                </div>
                <Badge variant={enabled ? "default" : "secondary"} className={enabled ? "bg-success/15 text-success border-success/20" : ""}>
                  {enabled ? "Enabled" : "Disabled"}
                </Badge>
              </div>
            );
          })}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Quiet Hours</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Status</span>
            <Badge variant="secondary">Disabled</Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Start</span>
            <span className="text-sm">22:00</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">End</span>
            <span className="text-sm">08:00</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Timezone</span>
            <span className="text-sm">{recipient.timezone ?? "UTC"}</span>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function RecipientSegments({ recipient }: { recipient: NonNullable<ReturnType<typeof useRecipient>> }) {
  const segments = recipient.segments ?? [];

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Assigned Segments</CardTitle>
        </CardHeader>
        <CardContent>
          {segments.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {segments.map((seg) => (
                <Badge key={seg} variant="outline" className="text-sm px-3 py-1">{seg}</Badge>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">No segments assigned.</p>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Tags</CardTitle>
        </CardHeader>
        <CardContent>
          {(recipient.tags ?? []).length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {(recipient.tags ?? []).map((tag) => (
                <Badge key={tag} variant="secondary" className="text-sm px-3 py-1">{tag}</Badge>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">No tags assigned.</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function RecipientNotifications({ recipientId }: { recipientId: string }) {
  const notifications = useRecipientNotifications(recipientId);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Recent Notifications</CardTitle>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>ID</TableHead>
              <TableHead>Channel</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Subject</TableHead>
              <TableHead>Created</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {notifications.map((n) => (
              <TableRow key={n.id}>
                <TableCell>
                  <Link href={`/deliveries/${n.id}`} className="font-mono text-xs hover:underline">
                    {n.id}
                  </Link>
                </TableCell>
                <TableCell><ChannelBadge channel={n.channel} showIcon /></TableCell>
                <TableCell><StatusBadge status={n.status} /></TableCell>
                <TableCell className="text-sm truncate max-w-[200px]">{n.subject ?? n.body}</TableCell>
                <TableCell className="text-xs text-muted-foreground whitespace-nowrap">
                  {format(new Date(n.createdAt), "MMM d, HH:mm")}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}

function RecipientDetail({ id }: { id: string }) {
  const recipient = useRecipient(id);
  const router = useRouter();

  if (!recipient) {
    return (
      <div className="flex flex-col items-center justify-center py-20">
        <h2 className="text-lg font-medium">Recipient not found</h2>
        <p className="text-sm text-muted-foreground mt-1">The recipient {id} does not exist.</p>
        <Button variant="outline" className="mt-4" onClick={() => router.push("/recipients")}>
          <ArrowLeft className="size-3.5 mr-1" /> Back to recipients
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title={recipient.name}
        description={recipient.email ?? recipient.id}
        breadcrumbs={[
          { label: "Dashboard", href: "/" },
          { label: "Recipients", href: "/recipients" },
          { label: recipient.name },
        ]}
        actions={
          <div className="flex items-center gap-2">
            <Button size="sm" variant="outline" className="gap-1.5">
              <ExternalLink className="size-3.5" /> Export
            </Button>
            <Button size="sm" variant="outline" className="gap-1.5 text-destructive">
              <Trash2 className="size-3.5" /> Delete
            </Button>
          </div>
        }
      />

      <Tabs defaultValue="overview">
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="devices">Devices ({recipient.devices?.length ?? 0})</TabsTrigger>
          <TabsTrigger value="preferences">Preferences</TabsTrigger>
          <TabsTrigger value="segments">Segments & Tags</TabsTrigger>
          <TabsTrigger value="history">Notification History</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="mt-4">
          <RecipientOverview recipient={recipient} />
        </TabsContent>

        <TabsContent value="devices" className="mt-4">
          <RecipientDevices recipient={recipient} />
        </TabsContent>

        <TabsContent value="preferences" className="mt-4">
          <RecipientPreferences recipient={recipient} />
        </TabsContent>

        <TabsContent value="segments" className="mt-4">
          <RecipientSegments recipient={recipient} />
        </TabsContent>

        <TabsContent value="history" className="mt-4">
          <RecipientNotifications recipientId={recipient.id} />
        </TabsContent>
      </Tabs>
    </div>
  );
}

export default function RecipientDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  return <RecipientDetail id={id} />;
}
