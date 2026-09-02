"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { NotificationSettings } from "@/components/custom/settings/notification-settings";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Globe, Bell, Shield, Palette, Save, Trash2, type LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";

type TabId = "general" | "notifications" | "security" | "appearance";

const TABS: { id: TabId; label: string; icon: LucideIcon }[] = [
  { id: "general", label: "General", icon: Globe },
  { id: "notifications", label: "Notifications", icon: Bell },
  { id: "security", label: "Security", icon: Shield },
  { id: "appearance", label: "Appearance", icon: Palette },
];

export default function SettingsPage() {
  const [tab, setTab] = useState<TabId>("general");
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
          tab === "general" ? (
            <Button size="sm" className="gap-1.5" onClick={handleSave}>
              <Save className="size-3.5" /> {saved ? "Saved!" : "Save changes"}
            </Button>
          ) : undefined
        }
      />

      <div className="grid gap-6 lg:grid-cols-[280px_1fr]">
        {/* Settings nav */}
        <Card className="h-fit">
          <CardContent className="pt-4 space-y-1">
            {TABS.map((item) => (
              <button
                key={item.id}
                onClick={() => setTab(item.id)}
                className={cn(
                  "flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
                  tab === item.id
                    ? "bg-muted/70 text-foreground"
                    : "text-muted-foreground hover:bg-muted/50",
                )}
              >
                <item.icon className="size-3.5" />
                <span>{item.label}</span>
              </button>
            ))}
          </CardContent>
        </Card>

        {/* Settings content */}
        {tab === "general" && (
          <div className="space-y-6">
            <Card>
              <CardContent className="pt-4 space-y-4">
                <div className="flex items-center gap-2 font-medium">
                  <Globe className="size-4 text-muted-foreground" /> Organization
                </div>
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
              <CardContent className="pt-4">
                <div className="flex items-center justify-between">
                  <div>
                    <div className="flex items-center gap-2 text-sm font-medium text-destructive">
                      <Trash2 className="size-4" /> Delete organization
                    </div>
                    <div className="text-xs text-muted-foreground">
                      Permanently delete this organization and all its data
                    </div>
                  </div>
                  <Button variant="destructive" size="sm">Delete</Button>
                </div>
              </CardContent>
            </Card>
          </div>
        )}

        {tab === "notifications" && <NotificationSettings />}

        {tab === "security" && (
          <Card>
            <CardContent className="pt-4">
              <div className="flex items-center gap-2 font-medium">
                <Shield className="size-4 text-muted-foreground" /> Security
              </div>
              <p className="text-sm text-muted-foreground">
                Security settings for your account and this project.
              </p>
            </CardContent>
          </Card>
        )}

        {tab === "appearance" && (
          <Card>
            <CardContent className="pt-4">
              <div className="flex items-center gap-2 font-medium">
                <Palette className="size-4 text-muted-foreground" /> Appearance
              </div>
              <p className="text-sm text-muted-foreground">
                Theme and display preferences.
              </p>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}
