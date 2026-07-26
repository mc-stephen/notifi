"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Globe,
  Bell,
  Shield,
  Palette,
  Save,
  Trash2,
} from "lucide-react";

export default function SettingsPage() {
  const [orgName, setOrgName] = useState("Acme Corp");
  const [orgSlug, setOrgSlug] = useState("acme-corp");
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div className="space-y-6">
      <PageHeader
        title="Settings"
        description="Organization and project settings"
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Settings" }]}
        actions={
          <Button size="sm" className="gap-1.5" onClick={handleSave}>
            <Save className="size-3.5" /> {saved ? "Saved!" : "Save changes"}
          </Button>
        }
      />

      <div className="grid gap-6 lg:grid-cols-[280px_1fr]">
        {/* Settings nav */}
        <Card className="h-fit">
          <CardContent className="pt-4 space-y-1">
            {[
              { icon: Globe, label: "General" },
              { icon: Bell, label: "Notifications" },
              { icon: Shield, label: "Security" },
              { icon: Palette, label: "Appearance" },
            ].map((item) => (
              <button
                key={item.label}
                className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-muted/50 transition-colors"
              >
                <item.icon className="size-3.5" />
                <span>{item.label}</span>
              </button>
            ))}
          </CardContent>
        </Card>

        {/* Settings content */}
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Globe className="size-4" /> Organization
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <label className="text-sm font-medium">Organization name</label>
                  <Input value={orgName} onChange={(e) => setOrgName(e.target.value)} />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">Slug</label>
                  <Input value={orgSlug} onChange={(e) => setOrgSlug(e.target.value)} />
                </div>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Bell className="size-4" /> Notification Preferences
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {[
                { label: "Email notifications for failures", description: "Get notified when a notification delivery fails", enabled: true },
                { label: "Weekly digest", description: "Receive a weekly summary of notification activity", enabled: true },
                { label: "SMS alerts for critical failures", description: "SMS alert when urgent notifications fail", enabled: false },
                { label: "Webhook for all events", description: "Send all events to your webhook endpoint", enabled: false },
              ].map((pref) => (
                <div key={pref.label} className="flex items-center justify-between">
                  <div>
                    <div className="text-sm font-medium">{pref.label}</div>
                    <div className="text-xs text-muted-foreground">{pref.description}</div>
                  </div>
                  <Badge variant={pref.enabled ? "default" : "secondary"} className={pref.enabled ? "bg-success/15 text-success border-success/20" : ""}>
                    {pref.enabled ? "Enabled" : "Disabled"}
                  </Badge>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-destructive">
                <Trash2 className="size-4" /> Danger Zone
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-sm font-medium">Delete organization</div>
                  <div className="text-xs text-muted-foreground">Permanently delete this organization and all its data</div>
                </div>
                <Button variant="destructive" size="sm">Delete</Button>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
