"use client";

import { use, useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { useNotification } from "@/hooks";
import { PageHeader } from "@/components/custom/page-header";
import { StatusBadge } from "@/components/custom/status-badge";
import { PriorityBadge } from "@/components/custom/priority-badge";
import { ChannelBadge } from "@/components/custom/channel-badge";
import { JsonViewer } from "@/components/custom/json-viewer";
import { CodeBlock } from "@/components/custom/code-block";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/separator";
import { format } from "date-fns";
import {
  RotateCcw,
  X,
  Copy,
  ArrowLeft,
  Clock,
  CheckCircle2,
  Send,
  Zap,
  Globe,
} from "lucide-react";
import Link from "next/link";

function NewNotificationForm() {
  const router = useRouter();
  const [channel, setChannel] = useState("email");
  const [recipientId, setRecipientId] = useState("");
  const [subject, setSubject] = useState("");
  const [body, setBody] = useState("");
  const [priority, setPriority] = useState("normal");

  return (
    <div className="space-y-6">
      <PageHeader
        title="Send Notification"
        description="Manually dispatch a notification to a recipient"
        breadcrumbs={[
          { label: "Dashboard", href: "/" },
          { label: "Deliveries", href: "/deliveries" },
          { label: "New" },
        ]}
        actions={
          <div className="flex items-center gap-2">
            <Button size="sm" variant="outline" onClick={() => router.push("/deliveries")}>
              <ArrowLeft className="size-3.5 mr-1" /> Cancel
            </Button>
            <Button size="sm" className="gap-1.5" onClick={() => router.push("/deliveries")}>
              <Send className="size-3.5" /> Send notification
            </Button>
          </div>
        }
      />

      <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
        <Card>
          <CardHeader>
            <CardTitle>Notification Details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-4 md:grid-cols-2">
              <div className="space-y-2">
                <label className="text-sm font-medium">Channel</label>
                <select
                  value={channel}
                  onChange={(e) => setChannel(e.target.value)}
                  className="w-full rounded-md border bg-transparent px-3 py-2 text-sm"
                >
                  <option value="email">Email</option>
                  <option value="sms">SMS</option>
                  <option value="push-android">Android Push</option>
                  <option value="push-ios">Apple Push</option>
                  <option value="web-push">Web Push</option>
                  <option value="slack">Slack</option>
                  <option value="discord">Discord</option>
                  <option value="webhook">Webhook</option>
                </select>
              </div>
              <div className="space-y-2">
                <label className="text-sm font-medium">Priority</label>
                <select
                  value={priority}
                  onChange={(e) => setPriority(e.target.value)}
                  className="w-full rounded-md border bg-transparent px-3 py-2 text-sm"
                >
                  <option value="low">Low</option>
                  <option value="normal">Normal</option>
                  <option value="high">High</option>
                  <option value="urgent">Urgent</option>
                </select>
              </div>
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Recipient ID</label>
              <Input
                value={recipientId}
                onChange={(e) => setRecipientId(e.target.value)}
                placeholder="e.g. rcp_0001"
              />
            </div>
            {channel === "email" && (
              <div className="space-y-2">
                <label className="text-sm font-medium">Subject</label>
                <Input
                  value={subject}
                  onChange={(e) => setSubject(e.target.value)}
                  placeholder="Email subject line"
                />
              </div>
            )}
            <div className="space-y-2">
              <label className="text-sm font-medium">Body</label>
              <textarea
                value={body}
                onChange={(e) => setBody(e.target.value)}
                placeholder="Notification body content..."
                className="w-full min-h-[200px] rounded-md border bg-transparent px-3 py-2 text-sm font-mono placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring resize-y"
              />
            </div>
          </CardContent>
        </Card>

        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-sm">Preview</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="space-y-1">
                <span className="text-xs text-muted-foreground">Channel</span>
                <p className="text-sm font-medium capitalize">{channel.replace(/-/g, " ")}</p>
              </div>
              <div className="space-y-1">
                <span className="text-xs text-muted-foreground">Priority</span>
                <p className="text-sm font-medium capitalize">{priority}</p>
              </div>
              <div className="space-y-1">
                <span className="text-xs text-muted-foreground">Recipient</span>
                <p className="text-sm font-mono text-xs">{recipientId || "—"}</p>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-sm flex items-center gap-2">
                <Zap className="size-3.5" /> Quick Tips
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 text-xs text-muted-foreground">
              <p>Use a valid recipient ID that exists in your project.</p>
              <p>The notification will be dispatched immediately through the configured channel.</p>
              <p>You can track delivery status from the deliveries list.</p>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}

function Timeline() {
  const steps = useMemo(() => {
    const allSteps = [
      { type: "queued", label: "Queued", icon: Clock, color: "text-info", time: "0s" },
      { type: "worker_assigned", label: "Worker Assigned", icon: Zap, color: "text-muted-foreground", time: "12ms" },
      { type: "provider_selected", label: "Provider Selected", icon: Globe, color: "text-muted-foreground", time: "45ms" },
      { type: "sent", label: "Sent", icon: Send, color: "text-info", time: "89ms" },
      { type: "delivered", label: "Delivered", icon: CheckCircle2, color: "text-success", time: "340ms" },
    ];

    const now = new Date();
    return allSteps.map((step, i) => ({
      ...step,
      timestamp: new Date(now.getTime() - (allSteps.length - i) * 2000).toISOString(),
    }));
  }, []);

  return (
    <div className="relative">
      <div className="absolute left-[15px] top-2 bottom-2 w-px bg-border" />
      <div className="space-y-0">
        {steps.map((step) => {
          const Icon = step.icon;
          return (
            <div key={step.type} className="relative flex items-start gap-4 py-3">
              <div className="relative z-10 flex size-8 shrink-0 items-center justify-center rounded-full bg-background border">
                <Icon className={`size-3.5 ${step.color}`} />
              </div>
              <div className="flex-1 min-w-0 pt-1">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium">{step.label}</span>
                  <span className="text-xs text-muted-foreground">{step.time}</span>
                </div>
                <span className="text-xs text-muted-foreground">
                  {format(new Date(step.timestamp), "MMM d, HH:mm:ss")}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function NotificationDetail({ id }: { id: string }) {
  const notification = useNotification(id);
  const router = useRouter();

  if (id === "new") {
    return <NewNotificationForm />;
  }

  if (!notification) {
    return (
      <div className="flex flex-col items-center justify-center py-20">
        <h2 className="text-lg font-medium">Notification not found</h2>
        <p className="text-sm text-muted-foreground mt-1">The notification {id} does not exist.</p>
        <Button variant="outline" className="mt-4" onClick={() => router.push("/deliveries")}>
          <ArrowLeft className="size-3.5 mr-1" /> Back to deliveries
        </Button>
      </div>
    );
  }

  const metadata = notification.metadata ?? {};
  const providerResponse = notification.providerResponse ?? {};

  return (
    <div className="space-y-6">
      <PageHeader
        title={notification.id}
        description={notification.subject ?? notification.body.slice(0, 80)}
        breadcrumbs={[
          { label: "Dashboard", href: "/" },
          { label: "Deliveries", href: "/deliveries" },
          { label: notification.id },
        ]}
        actions={
          <div className="flex items-center gap-2">
            {notification.status === "failed" && (
              <Button size="sm" variant="outline" className="gap-1.5">
                <RotateCcw className="size-3.5" /> Retry
              </Button>
            )}
            {notification.status !== "cancelled" && (
              <Button size="sm" variant="outline" className="gap-1.5 text-destructive">
                <X className="size-3.5" /> Cancel
              </Button>
            )}
            <Button size="sm" variant="outline" className="gap-1.5">
              <Copy className="size-3.5" /> Duplicate
            </Button>
          </div>
        }
      />

      <Tabs defaultValue="overview">
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="timeline">Timeline</TabsTrigger>
          <TabsTrigger value="payload">Payload</TabsTrigger>
          <TabsTrigger value="metadata">Metadata</TabsTrigger>
          <TabsTrigger value="provider">Provider Response</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="mt-4">
          <div className="grid gap-6 lg:grid-cols-3">
            <Card className="lg:col-span-2">
              <CardHeader>
                <CardTitle>Details</CardTitle>
              </CardHeader>
              <CardContent>
                <dl className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <dt className="text-muted-foreground">Status</dt>
                    <dd className="mt-1"><StatusBadge status={notification.status} /></dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Priority</dt>
                    <dd className="mt-1"><PriorityBadge priority={notification.priority} /></dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Channel</dt>
                    <dd className="mt-1"><ChannelBadge channel={notification.channel} /></dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Recipient</dt>
                    <dd className="mt-1">
                      <Link href={`/recipients/${notification.recipientId}`} className="font-mono text-xs hover:underline">
                        {notification.recipientId}
                      </Link>
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">Created</dt>
                    <dd className="mt-1 text-sm">{format(new Date(notification.createdAt), "MMM d, yyyy HH:mm:ss")}</dd>
                  </div>
                  {notification.sentAt && (
                    <div>
                      <dt className="text-muted-foreground">Sent</dt>
                      <dd className="mt-1 text-sm">{format(new Date(notification.sentAt), "MMM d, yyyy HH:mm:ss")}</dd>
                    </div>
                  )}
                  {notification.deliveredAt && (
                    <div>
                      <dt className="text-muted-foreground">Delivered</dt>
                      <dd className="mt-1 text-sm">{format(new Date(notification.deliveredAt), "MMM d, yyyy HH:mm:ss")}</dd>
                    </div>
                  )}
                  <div>
                    <dt className="text-muted-foreground">Retry Count</dt>
                    <dd className="mt-1 text-sm">{notification.retryCount}</dd>
                  </div>
                  {notification.templateId && (
                    <div>
                      <dt className="text-muted-foreground">Template</dt>
                      <dd className="mt-1">
                        <Link href={`/templates/${notification.templateId}`} className="font-mono text-xs hover:underline">
                          {notification.templateId}
                        </Link>
                      </dd>
                    </div>
                  )}
                  {notification.providerId && (
                    <div>
                      <dt className="text-muted-foreground">Provider</dt>
                      <dd className="mt-1 font-mono text-xs">{notification.providerId}</dd>
                    </div>
                  )}
                </dl>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Quick Info</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">Subject</span>
                  <span className="text-sm truncate max-w-[150px]">{notification.subject ?? "—"}</span>
                </div>
                <Separator />
                <div>
                  <span className="text-sm text-muted-foreground">Body</span>
                  <p className="text-sm mt-1 whitespace-pre-wrap break-words">{notification.body}</p>
                </div>
                {notification.failureReason && (
                  <>
                    <Separator />
                    <div>
                      <span className="text-sm text-destructive font-medium">Failure Reason</span>
                      <p className="text-sm mt-1 text-destructive/80">{notification.failureReason}</p>
                    </div>
                  </>
                )}
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="timeline" className="mt-4">
          <Card>
            <CardHeader>
              <CardTitle>Delivery Timeline</CardTitle>
            </CardHeader>
            <CardContent>
              <Timeline />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="payload" className="mt-4">
          <Card>
            <CardHeader>
              <CardTitle>Payload</CardTitle>
            </CardHeader>
            <CardContent>
              <CodeBlock code={JSON.stringify({ subject: notification.subject, body: notification.body, channel: notification.channel, priority: notification.priority }, null, 2)} language="JSON" />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="metadata" className="mt-4">
          <Card>
            <CardHeader>
              <CardTitle>Metadata</CardTitle>
            </CardHeader>
            <CardContent>
              {Object.keys(metadata).length > 0 ? (
                <JsonViewer data={metadata} />
              ) : (
                <p className="text-sm text-muted-foreground">No metadata available.</p>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="provider" className="mt-4">
          <Card>
            <CardHeader>
              <CardTitle>Provider Response</CardTitle>
            </CardHeader>
            <CardContent>
              {Object.keys(providerResponse).length > 0 ? (
                <JsonViewer data={providerResponse} />
              ) : (
                <p className="text-sm text-muted-foreground">No provider response available.</p>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}

export default function NotificationDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  return <NotificationDetail id={id} />;
}
