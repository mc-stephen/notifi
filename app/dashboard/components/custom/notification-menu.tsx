"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import {
  Bell,
  BellRing,
  Check,
  type LucideIcon,
  UserPlus,
  UserMinus,
  UserCog,
  Link,
  Link2Off,
  KeyRound,
  FolderPlus,
  CreditCard,
  LogIn,
  Megaphone,
  Sparkles,
  Rocket,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import type { InAppNotification, InAppNotificationType } from "@/lib/types";

const TYPE_ICONS: Record<InAppNotificationType, LucideIcon> = {
  team_add: UserPlus,
  team_remove: UserMinus,
  role_change: UserCog,
  provider_add: Link,
  provider_delete: Link2Off,
  api_key_created: KeyRound,
  api_key_revoked: KeyRound,
  project_created: FolderPlus,
  billing_change: CreditCard,
  system: Check,
  new_login: LogIn,
  marketing: Megaphone,
  billing: CreditCard,
  welcome: Sparkles,
  update: Rocket,
};

const TYPE_TILES: Record<InAppNotificationType, string> = {
  team_add: "bg-success/10 text-success",
  team_remove: "bg-destructive/10 text-destructive",
  role_change: "bg-info/10 text-info",
  provider_add: "bg-primary/10 text-primary",
  provider_delete: "bg-destructive/10 text-destructive",
  api_key_created: "bg-warning/10 text-warning",
  api_key_revoked: "bg-muted-foreground/10 text-muted-foreground",
  project_created: "bg-info/10 text-info",
  billing_change: "bg-warning/10 text-warning",
  system: "bg-muted text-muted-foreground",
  new_login: "bg-info/10 text-info",
  marketing: "bg-primary/10 text-primary",
  billing: "bg-warning/10 text-warning",
  welcome: "bg-success/10 text-success",
  update: "bg-info/10 text-info",
};

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  return `${days}d ago`;
}

export function NotificationMenu() {
  const router = useRouter();
  const [notifications, setNotifications] = useState<InAppNotification[]>([]);
  const [unread, setUnread] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    async function load() {
      try {
        const res = await fetch("/v1/notifications?limit=10", { signal: controller.signal });
        if (!res.ok) return;
        const data = await res.json();
        setNotifications(data.notifications ?? []);
      } catch {
        // silent
      }
    }
    async function loadCount() {
      try {
        const res = await fetch("/v1/notifications/count", { signal: controller.signal });
        if (!res.ok) return;
        const data = await res.json();
        setUnread(data.count ?? 0);
      } catch {
        // silent
      }
    }
    load();
    loadCount();
    return () => controller.abort();
  }, []);

  const markAllRead = async () => {
    setNotifications((prev) => prev.map((n) => ({ ...n, read: true, readAt: new Date().toISOString() })));
    setUnread(0);
    try {
      await fetch("/v1/notifications/read-all", { method: "PATCH" });
    } catch {
      // silent
    }
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            size="icon-sm"
            className="relative text-muted-foreground hover:text-foreground"
          />
        }
      >
        <Bell className="size-4" />
        {unread > 0 && (
          <span className="absolute -top-0.5 -right-0.5 size-2 rounded-full bg-primary" />
        )}
        <span className="sr-only">Notifications</span>
      </DropdownMenuTrigger>

      <DropdownMenuContent align="end" className="w-96 p-0">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3">
          <div className="flex items-center gap-2">
            <BellRing className="size-4 text-muted-foreground" />
            <span className="text-sm font-semibold">Notifications</span>
            {unread > 0 && (
              <span className="flex size-5 items-center justify-center rounded-full bg-primary text-[10px] font-semibold text-primary-foreground">
                {unread}
              </span>
            )}
          </div>
          {unread > 0 && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 gap-1 px-2 text-xs text-muted-foreground hover:text-foreground"
              onClick={markAllRead}
            >
              <Check className="size-3.5" />
              Mark all read
            </Button>
          )}
        </div>

        <DropdownMenuSeparator className="my-0" />

        {/* Items */}
        <div className="max-h-80 overflow-y-auto p-1.5">
          {notifications.length === 0 && (
            <div className="px-4 py-8 text-center text-sm text-muted-foreground">
              No notifications yet.
            </div>
          )}
          {notifications.map((n) => {
            const Icon = TYPE_ICONS[n.type];
            return (
              <DropdownMenuItem
                key={n.id}
                className={cn(
                  "items-start gap-3 rounded-lg px-2.5 py-2.5",
                  !n.read && "bg-primary/[0.04]",
                )}
                onClick={() => router.push("/notifications")}
              >
                <div
                  className={cn(
                    "mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg",
                    TYPE_TILES[n.type],
                  )}
                >
                  <Icon className="size-4" />
                </div>
                <div className="flex min-w-0 flex-1 flex-col">
                  <div className="flex items-center justify-between gap-2">
                    <span className={cn("truncate text-sm font-medium", !n.read && "font-semibold")}>
                      {n.title}
                    </span>
                    <span className="shrink-0 text-[10px] uppercase tracking-wider text-muted-foreground/70">
                      {timeAgo(n.createdAt)}
                    </span>
                  </div>
                  <span className="truncate text-xs text-muted-foreground">
                    {n.content}
                  </span>
                </div>
                {!n.read && (
                  <span className="mt-2 size-1.5 shrink-0 rounded-full bg-primary" />
                )}
              </DropdownMenuItem>
            );
          })}
        </div>

        <DropdownMenuSeparator className="my-0" />

        {/* Footer */}
        <div className="p-1.5">
          <DropdownMenuItem
            className="justify-center rounded-lg py-2 text-sm font-medium text-primary"
            onClick={() => router.push("/notifications")}
          >
            View all notifications
          </DropdownMenuItem>
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
