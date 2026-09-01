"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { links } from "@/lib/env";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Link,
  CheckCircle2,
  Plus,
  ExternalLink,
  Search,
  Plug,
} from "lucide-react";

type Integration = {
  id: string;
  name: string;
  category: string;
  description: string;
  icon: string;
  connected: boolean;
  eventsSupported: number;
  docs: string;
};

const INTEGRATIONS: Integration[] = [
  { id: "segment", name: "Segment", category: "Analytics", description: "Send notification events to Segment for analytics and audience building.", icon: "S", connected: true, eventsSupported: 12, docs: links.integrationDocs.segment },
  { id: "posthog", name: "PostHog", category: "Analytics", description: "Track notification events in PostHog for product analytics.", icon: "P", connected: false, eventsSupported: 8, docs: links.integrationDocs.posthog },
  { id: "datadog", name: "Datadog", category: "Monitoring", description: "Forward notification metrics and logs to Datadog.", icon: "D", connected: true, eventsSupported: 15, docs: links.integrationDocs.datadog },
  { id: "sentry", name: "Sentry", category: "Error Tracking", description: "Capture notification delivery errors in Sentry.", icon: "S", connected: false, eventsSupported: 6, docs: links.integrationDocs.sentry },
  { id: "zapier", name: "Zapier", category: "Automation", description: "Trigger Zaps from notification events for workflow automation.", icon: "Z", connected: false, eventsSupported: 20, docs: links.integrationDocs.zapier },
  { id: "mixpanel", name: "Mixpanel", category: "Analytics", description: "Track notification engagement in Mixpanel funnels.", icon: "M", connected: false, eventsSupported: 10, docs: links.integrationDocs.mixpanel },
  { id: "slack", name: "Slack", category: "Communication", description: "Send notification alerts to Slack channels.", icon: "S", connected: true, eventsSupported: 5, docs: links.integrationDocs.slack },
  { id: "pagerduty", name: "PagerDuty", category: "Incident Management", description: "Create PagerDuty incidents from critical notification failures.", icon: "P", connected: false, eventsSupported: 4, docs: links.integrationDocs.pagerduty },
  { id: "opentelemetry", name: "OpenTelemetry", category: "Observability", description: "Export notification traces and metrics via OTLP.", icon: "O", connected: false, eventsSupported: 18, docs: links.integrationDocs.opentelemetry },
];

const CATEGORY_COLORS: Record<string, string> = {
  Analytics: "bg-info/15 text-info",
  Monitoring: "bg-success/15 text-success",
  "Error Tracking": "bg-destructive/15 text-destructive",
  Automation: "bg-warning/15 text-warning",
  Communication: "bg-primary/15 text-primary",
  "Incident Management": "bg-destructive/15 text-destructive",
  Observability: "bg-muted text-muted-foreground",
};

export default function IntegrationsPage() {
  const [search, setSearch] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [connectDialog, setConnectDialog] = useState<Integration | null>(null);

  const categories = [...new Set(INTEGRATIONS.map((i) => i.category))];

  const filtered = INTEGRATIONS.filter((i) => {
    if (search && !i.name.toLowerCase().includes(search.toLowerCase())) return false;
    if (selectedCategory && i.category !== selectedCategory) return false;
    return true;
  });

  const connectedCount = INTEGRATIONS.filter((i) => i.connected).length;

  return (
    <div className="space-y-6">
      <PageHeader
        title="Integrations"
        description={`${connectedCount} connected integrations`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Integrations" }]}
      />

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Total Integrations</span>
              <Plug className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{INTEGRATIONS.length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Connected</span>
              <CheckCircle2 className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{connectedCount}</div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Categories</span>
              <Link className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">{categories.length}</div>
          </CardContent>
        </Card>
      </div>

      <div className="flex items-center gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
          <Input
            placeholder="Search integrations..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-8 h-8 text-sm"
          />
        </div>
        <div className="flex items-center gap-2 flex-wrap">
          {categories.map((cat) => (
            <button
              key={cat}
              onClick={() => setSelectedCategory(selectedCategory === cat ? null : cat)}
              className={`rounded-md border px-2.5 py-1 text-xs transition-colors ${
                selectedCategory === cat ? "border-primary bg-primary/5" : "hover:bg-muted/50"
              }`}
            >
              {cat}
            </button>
          ))}
        </div>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {filtered.map((integration) => (
          <Card key={integration.id} className="hover:bg-muted/30 transition-colors">
            <CardHeader className="pb-3">
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-3">
                  <div className="flex size-10 items-center justify-center rounded-lg bg-muted text-sm font-bold">
                    {integration.icon}
                  </div>
                  <div>
                    <CardTitle className="text-sm">{integration.name}</CardTitle>
                    <Badge variant="secondary" className={`text-[10px] mt-0.5 ${CATEGORY_COLORS[integration.category] ?? ""}`}>
                      {integration.category}
                    </Badge>
                  </div>
                </div>
                {integration.connected ? (
                  <Badge variant="default" className="bg-success/15 text-success border-success/20 text-[10px]">
                    Connected
                  </Badge>
                ) : (
                  <Badge variant="secondary" className="text-[10px]">
                    Not connected
                  </Badge>
                )}
              </div>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-xs text-muted-foreground">{integration.description}</p>
              <div className="flex items-center justify-between text-xs">
                <span className="text-muted-foreground">{integration.eventsSupported} events supported</span>
                <a
                  href={integration.docs}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1"
                >
                  Docs <ExternalLink className="size-2.5" />
                </a>
              </div>
              <div className="flex gap-2">
                {integration.connected ? (
                  <>
                    <Button size="sm" variant="outline" className="flex-1 text-xs">
                      Configure
                    </Button>
                    <Button size="sm" variant="ghost" className="text-destructive text-xs">
                      Disconnect
                    </Button>
                  </>
                ) : (
                  <Button size="sm" className="flex-1 gap-1.5" onClick={() => setConnectDialog(integration)}>
                    <Plus className="size-3" /> Connect
                  </Button>
                )}
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <Dialog open={!!connectDialog} onOpenChange={() => setConnectDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Connect {connectDialog?.name}</DialogTitle>
            <DialogDescription>
              Configure the {connectDialog?.name} integration for your project.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">API Key</label>
              <Input placeholder={`Enter your ${connectDialog?.name} API key`} type="password" />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Project / Workspace ID</label>
              <Input placeholder={`Enter your ${connectDialog?.name} project ID`} />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Events to forward</label>
              <div className="space-y-1">
                {["notification.sent", "notification.delivered", "notification.failed", "notification.opened"].map((event) => (
                  <label key={event} className="flex items-center gap-2 text-sm">
                    <input type="checkbox" defaultChecked className="rounded" />
                    <code className="text-xs font-mono">{event}</code>
                  </label>
                ))}
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConnectDialog(null)}>Cancel</Button>
            <Button onClick={() => setConnectDialog(null)}>Connect</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
