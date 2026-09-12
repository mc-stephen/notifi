"use client";

import { useState, useSyncExternalStore } from "react";
import { useTheme } from "next-themes";
import { PageHeader } from "@/components/custom/page-header";
import { NotificationSettings } from "@/components/custom/settings/notification-settings";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  Globe,
  Bell,
  Palette,
  Save,
  Trash2,
  Monitor,
  Moon,
  Sun,
  Laptop,
  type LucideIcon,
} from "lucide-react";
import { cn } from "@/lib/utils";

type TabId = "general" | "notifications" | "appearance";

const TABS: { id: TabId; label: string; icon: LucideIcon }[] = [
  { id: "general", label: "General", icon: Globe },
  { id: "notifications", label: "Notifications", icon: Bell },
  { id: "appearance", label: "Appearance", icon: Palette },
];

const TIMEZONES = [
  "UTC",
  "Africa/Lagos",
  "Europe/London",
  "America/New_York",
  "Asia/Singapore",
];

const THEMES = [
  { id: "light", label: "Light", icon: Sun },
  { id: "dark", label: "Dark", icon: Moon },
  { id: "system", label: "System", icon: Laptop },
] as const;

export default function SettingsPage() {
  const [tab, setTab] = useState<TabId>("general");
  const [orgName, setOrgName] = useState("Acme Corp");
  const [orgSlug, setOrgSlug] = useState("acme-corp");
  const [orgEmail, setOrgEmail] = useState("hello@acme-corp.com");
  const [timezone, setTimezone] = useState("Africa/Lagos");
  const [defaultEnv, setDefaultEnv] = useState<"development" | "production">("development");
  const [retention, setRetention] = useState("90");
  const [saved, setSaved] = useState(false);
  const [compactMode, setCompactMode] = useState(false);

  const { theme, setTheme } = useTheme();
  const mounted = useSyncExternalStore(
    () => () => {},
    () => true,
    () => false
  );

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
                  <div className="space-y-2">
                    <label className="text-sm font-medium">Contact email</label>
                    <Input
                      type="email"
                      value={orgEmail}
                      onChange={(e) => setOrgEmail(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <label className="text-sm font-medium">Timezone</label>
                    <select
                      className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm focus-visible:border-ring focus-visible:outline-none"
                      value={timezone}
                      onChange={(e) => setTimezone(e.target.value)}
                    >
                      {TIMEZONES.map((tz) => (
                        <option key={tz} value={tz}>{tz}</option>
                      ))}
                    </select>
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="pt-4 space-y-4">
                <div className="flex items-center gap-2 font-medium">
                  <Monitor className="size-4 text-muted-foreground" /> Project defaults
                </div>
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <label className="text-sm font-medium">Default environment</label>
                    <div className="flex items-center gap-1 rounded-lg border border-border bg-card p-1" role="group" aria-label="Default environment">
                      {(["development", "production"] as const).map((env) => (
                        <button
                          key={env}
                          type="button"
                          onClick={() => setDefaultEnv(env)}
                          aria-pressed={defaultEnv === env}
                          className={cn(
                            "rounded-md px-3 py-1.5 text-xs font-medium capitalize transition-colors",
                            defaultEnv === env
                              ? "bg-primary text-primary-foreground"
                              : "text-muted-foreground hover:text-foreground hover:bg-muted",
                          )}
                        >
                          {env}
                        </button>
                      ))}
                    </div>
                  </div>
                  <div className="space-y-2">
                    <label className="text-sm font-medium">Data retention (days)</label>
                    <select
                      className="flex h-9 w-full rounded-lg border border-input bg-card px-3 text-sm focus-visible:border-ring focus-visible:outline-none"
                      value={retention}
                      onChange={(e) => setRetention(e.target.value)}
                    >
                      {["30", "90", "180", "365"].map((days) => (
                        <option key={days} value={days}>{days} days</option>
                      ))}
                    </select>
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

        {tab === "appearance" && (
          <div className="space-y-6">
            <Card>
              <CardContent className="pt-4 space-y-4">
                <div className="flex items-center gap-2 font-medium">
                  <Palette className="size-4 text-muted-foreground" /> Theme
                </div>
                <div className="grid grid-cols-3 gap-3">
                  {THEMES.map((item) => (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => setTheme(item.id)}
                      aria-pressed={mounted && theme === item.id}
                      className={cn(
                        "flex flex-col items-center gap-2 rounded-lg border p-4 transition-colors",
                        mounted && theme === item.id
                          ? "border-primary bg-primary/5"
                          : "hover:border-muted-foreground/30",
                      )}
                    >
                      <item.icon className="size-5 text-muted-foreground" />
                      <span className="text-xs font-medium">{item.label}</span>
                    </button>
                  ))}
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="pt-4">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <div className="text-sm font-medium">Compact tables</div>
                    <div className="text-xs text-muted-foreground">
                      Reduce row padding across data tables
                    </div>
                  </div>
                  <Switch checked={compactMode} onCheckedChange={setCompactMode} />
                </div>
              </CardContent>
            </Card>
          </div>
        )}
      </div>
    </div>
  );
}
