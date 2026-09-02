"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import {
  Bell,
  BellRing,
  Check,
  Mail,
  MessageSquare,
  Webhook,
  Zap,
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

type Notification = {
  id: string;
  icon: typeof Mail;
  tileClass: string;
  title: string;
  snippet: string;
  time: string;
  unread: boolean;
};

// TODO(dashboard): replace with real notifications once the backend exposes
// the notifications list endpoint (M2+).
const MOCK_NOTIFICATIONS: Notification[] = [
  {
    id: "n_1",
    icon: Mail,
    tileClass: "bg-info/10 text-info",
    title: "Campaign delivered",
    snippet: "“Welcome Series” reached 12,847 recipients",
    time: "2m ago",
    unread: true,
  },
  {
    id: "n_2",
    icon: Zap,
    tileClass: "bg-warning/10 text-warning",
    title: "Webhook failing",
    snippet: "orders.sync is returning 500s — retries queued",
    time: "1h ago",
    unread: true,
  },
  {
    id: "n_3",
    icon: MessageSquare,
    tileClass: "bg-success/10 text-success",
    title: "New SMS provider connected",
    snippet: "Twilio sandbox credentials verified",
    time: "3h ago",
    unread: false,
  },
  {
    id: "n_4",
    icon: Webhook,
    tileClass: "bg-primary/10 text-primary",
    title: "Rate limit bumped",
    snippet: "Production key raised to 1,000 req/min",
    time: "1d ago",
    unread: false,
  },
];

export function NotificationMenu() {
  const router = useRouter();
  const [notifications, setNotifications] = useState(MOCK_NOTIFICATIONS);
  const unread = notifications.filter((n) => n.unread).length;

  const markAllRead = () =>
    setNotifications((prev) => prev.map((n) => ({ ...n, unread: false })));

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
          {notifications.map((n) => {
            const Icon = n.icon;
            return (
              <DropdownMenuItem
                key={n.id}
                className={cn(
                  "items-start gap-3 rounded-lg px-2.5 py-2.5",
                  n.unread && "bg-primary/[0.04]",
                )}
              >
                <div
                  className={cn(
                    "mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg",
                    n.tileClass,
                  )}
                >
                  <Icon className="size-4" />
                </div>
                <div className="flex min-w-0 flex-1 flex-col">
                  <div className="flex items-center justify-between gap-2">
                    <span className="truncate text-sm font-medium">
                      {n.title}
                    </span>
                    <span className="shrink-0 text-[10px] uppercase tracking-wider text-muted-foreground/70">
                      {n.time}
                    </span>
                  </div>
                  <span className="truncate text-xs text-muted-foreground">
                    {n.snippet}
                  </span>
                </div>
                {n.unread && (
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
            onClick={() => router.push("/deliveries")}
          >
            View all notifications
          </DropdownMenuItem>
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
