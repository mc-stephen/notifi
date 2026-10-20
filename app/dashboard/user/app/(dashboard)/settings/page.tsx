"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { NotificationSettings } from "@/components/custom/settings/notification-settings";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Globe,
  Bell,
  Save,
  Trash2,
  Monitor,
  Check,
  type LucideIcon,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { api, ApiError } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";
import type { Project } from "@/lib/types";
import { toast } from "sonner";

type TabId = "general" | "notifications";

const TABS: { id: TabId; label: string; icon: LucideIcon }[] = [
  { id: "general", label: "General", icon: Globe },
  { id: "notifications", label: "Notifications", icon: Bell },
];

const STATE_HINTS = {
  active: "Notifications deliver normally.",
  inactive: "All delivery paused — API calls are rejected.",
} as const;

export default function SettingsPage() {
  const [tab, setTab] = useState<TabId>("general");
  const [orgName, setOrgName] = useState("Acme Corp");
  const [orgEmail, setOrgEmail] = useState("hello@acme-corp.com");
  const [switching, setSwitching] = useState(false);
  const [saved, setSaved] = useState(false);

  const { currentProject, updateProject } = useProjectStore();

  const setProjectActive = async (active: boolean) => {
    if (!currentProject || switching || currentProject.active === active) return;
    setSwitching(true);
    try {
      const { project } = await api<{ project: Project }>(
        `/app/projects/${currentProject.id}/active`,
        { method: "PATCH", body: JSON.stringify({ active }) },
      );
      updateProject({ ...currentProject, active: project.active });
      toast.success(active ? "Project active" : "Project paused — all delivery stopped");
    } catch (e) {
      toast.error(
        e instanceof ApiError ? e.message : "Couldn't change project state",
      );
    } finally {
      setSwitching(false);
    }
  };

  const handleSave = () => {
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div className="space-y-6">
      <PageHeader
        title="Settings"
        description="Organization and project settings"
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
                    <label className="text-sm font-medium">Contact email</label>
                    <Input
                      type="email"
                      value={orgEmail}
                      onChange={(e) => setOrgEmail(e.target.value)}
                    />
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardContent className="pt-4 space-y-4">
                <div className="flex items-center gap-2 font-medium">
                  <Monitor className="size-4 text-muted-foreground" /> Project defaults
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">Project state</label>
                  <div className="grid gap-3 sm:grid-cols-2" role="radiogroup" aria-label="Project state">
                    {([true, false] as const).map((value) => {
                      const selected = (currentProject?.active ?? false) === value;
                      return (
                        <button
                          key={String(value)}
                          type="button"
                          role="radio"
                          aria-checked={selected}
                          disabled={switching || !currentProject}
                          onClick={() => void setProjectActive(value)}
                          className={cn(
                            "flex items-start gap-3 rounded-lg border p-3 text-left transition-colors",
                            selected
                              ? "border-primary bg-primary/5"
                              : "hover:border-muted-foreground/30",
                          )}
                        >
                          <span className={cn("mt-1 size-2 shrink-0 rounded-full", value ? "bg-success" : "bg-destructive")} />
                          <span className="min-w-0 flex-1">
                            <span className="block text-sm font-medium">{value ? "Active" : "Inactive"}</span>
                            <span className="mt-0.5 block text-xs text-muted-foreground">{value ? STATE_HINTS.active : STATE_HINTS.inactive}</span>
                          </span>
                          {selected && <Check className="size-4 shrink-0 text-primary" />}
                        </button>
                      );
                    })}
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card className="border-destructive/20 bg-destructive/[0.03]">
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-destructive">
                  <Trash2 className="size-4" />
                  Delete organization
                </CardTitle>
                <CardDescription>
                  Permanently delete this organization and all its data. This action cannot be undone.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <Button variant="destructive" size="sm">Delete organization</Button>
              </CardContent>
            </Card>
          </div>
        )}
        {tab === "notifications" && <NotificationSettings />}
      </div>
    </div>
  );
}
