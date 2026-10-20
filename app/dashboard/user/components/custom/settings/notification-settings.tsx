"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Save, Lock, ShieldCheck } from "lucide-react";
import { useNotificationSettings } from "@/hooks";
import { CURRENT_USER_ROLE, ROLE_LABELS } from "@/lib/constants";
import { cn } from "@/lib/utils";

const CAN_EDIT = CURRENT_USER_ROLE === "owner" || CURRENT_USER_ROLE === "admin";

export function NotificationSettings() {
  const { categories, toggle, setAll } = useNotificationSettings();
  const [saved, setSaved] = useState(false);

  const handleSave = () => {
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-semibold">Notification Settings</h2>
          <p className="text-sm text-muted-foreground">
            Choose which project events notify you, and on which channels.
          </p>
        </div>
        {CAN_EDIT && (
          <Button size="sm" className="gap-1.5" onClick={handleSave}>
            <Save className="size-3.5" /> {saved ? "Saved!" : "Save changes"}
          </Button>
        )}
      </div>

      {!CAN_EDIT && (
        <div className="flex items-center gap-2 rounded-lg border border-muted bg-muted/40 px-3 py-2.5 text-sm text-muted-foreground">
          <Lock className="size-4" />
          <span>
            You have view access only. {ROLE_LABELS[CURRENT_USER_ROLE]}s cannot edit these settings — an owner or admin can.
          </span>
        </div>
      )}

      <div className="space-y-6">
        {categories.map((cat) => {
          const allOn = cat.settings.every((s) => s.email && s.inApp);
          return (
            <Card key={cat.id}>
              <CardHeader>
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <CardTitle className="flex items-center gap-2">
                      {cat.label}
                      {!CAN_EDIT && <ShieldCheck className="size-4 text-muted-foreground" />}
                    </CardTitle>
                    <CardDescription>{cat.description}</CardDescription>
                  </div>
                  {CAN_EDIT && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-7 px-2 text-xs"
                      onClick={() => setAll(cat.id, !allOn)}
                    >
                      {allOn ? "Disable all" : "Enable all"}
                    </Button>
                  )}
                </div>
              </CardHeader>
              <CardContent className="space-y-1">
                {cat.settings.map((s) => (
                  <div
                    key={s.key}
                    className={cn(
                      "flex flex-wrap items-center justify-between gap-3 rounded-lg px-2 py-2.5",
                      !CAN_EDIT && "opacity-70",
                    )}
                  >
                    <div className="min-w-0">
                      <div className="text-sm font-medium">{s.label}</div>
                      <div className="text-xs text-muted-foreground">{s.description}</div>
                    </div>
                    <div className="flex items-center gap-6">
                      <label className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Switch
                          size="sm"
                          checked={s.email}
                          disabled={!CAN_EDIT}
                          onCheckedChange={() => toggle(cat.id, s.key, "email")}
                        />
                        Email
                      </label>
                      <label className="flex items-center gap-2 text-sm text-muted-foreground">
                        <Switch
                          size="sm"
                          checked={s.inApp}
                          disabled={!CAN_EDIT}
                          onCheckedChange={() => toggle(cat.id, s.key, "inApp")}
                        />
                        In-App
                      </label>
                    </div>
                  </div>
                ))}
              </CardContent>
            </Card>
          );
        })}
      </div>

      <p className="text-xs text-muted-foreground">
        Settings apply to actions performed by team members in this project.
      </p>
    </div>
  );
}
