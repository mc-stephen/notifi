"use client";

import { useRouter } from "next/navigation";
import { Bell, BellRing, Mail, MessageSquare, Webhook, Zap } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";

// TODO(dashboard): replace with real notifications once the backend exposes
// the notifications list endpoint (M2+).
const MOCK_NOTIFICATIONS = [
  {
    id: "n_1",
    icon: Mail,
    title: "Campaign delivered",
    snippet: "“Welcome Series” reached 12,847 recipients",
    time: "2m ago",
    unread: true,
  },
  {
    id: "n_2",
    icon: Zap,
    title: "Webhook failing",
    snippet: "orders.sync is returning 500s — retries queued",
    time: "1h ago",
    unread: true,
  },
  {
    id: "n_3",
    icon: MessageSquare,
    title: "New SMS provider connected",
    snippet: "Twilio sandbox credentials verified",
    time: "3h ago",
    unread: false,
  },
  {
    id: "n_4",
    icon: Webhook,
    title: "Rate limit bumped",
    snippet: "Production key raised to 1,000 req/min",
    time: "1d ago",
    unread: false,
  },
];

export function NotificationMenu() {
  const router = useRouter();
  const unread = MOCK_NOTIFICATIONS.some((n) => n.unread);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button variant="ghost" size="icon-sm" className="relative" />
        }
      >
        <Bell className="size-4" />
        {unread && (
          <span className="absolute -top-0.5 -right-0.5 size-2 rounded-full bg-primary" />
        )}
        <span className="sr-only">Notifications</span>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-80">
        <DropdownMenuLabel>
          <div className="flex items-center gap-2">
            <BellRing className="size-4" />
            Notifications
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        {MOCK_NOTIFICATIONS.map((n) => {
          const Icon = n.icon;
          return (
            <DropdownMenuItem key={n.id} className="items-start gap-3 py-2.5">
              <div className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                <Icon className="size-4" />
              </div>
              <div className="flex min-w-0 flex-col">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">{n.title}</span>
                  {n.unread && (
                    <span className="size-1.5 shrink-0 rounded-full bg-primary" />
                  )}
                </div>
                <span className="text-xs text-muted-foreground truncate">
                  {n.snippet}
                </span>
                <span className="mt-0.5 text-[10px] uppercase tracking-wider text-muted-foreground/70">
                  {n.time}
                </span>
              </div>
            </DropdownMenuItem>
          );
        })}
        <DropdownMenuSeparator />
        <DropdownMenuItem
          className="justify-center text-sm font-medium text-primary"
          onClick={() => router.push("/notifications")}
        >
          View all notifications
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
