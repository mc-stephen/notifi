"use client";

import { useState } from "react";
import { WEBHOOK_EVENTS } from "@/lib/constants";
import { cn } from "@/lib/utils";
import { Bell, ChevronDown, Contact, FileText, KeyRound, Megaphone, Plug, Users, type LucideIcon } from "lucide-react";

const GROUP_ICONS: Record<string, LucideIcon> = {
  Notifications: Bell,
  Campaigns: Megaphone,
  Providers: Plug,
  "API Keys": KeyRound,
  Teams: Users,
  Recipients: Contact,
  Templates: FileText,
};

export function AvailableWebhookEvents() {
  const [open, setOpen] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(WEBHOOK_EVENTS.map((g) => [g.label, true])),
  );

  const toggle = (label: string) =>
    setOpen((prev) => ({ ...prev, [label]: !prev[label] }));

  return (
    <section className="space-y-4">
      <div>
        <h2 className="text-base font-semibold">Available Webhook Events</h2>
        <p className="mt-0.5 text-sm text-muted-foreground">
          Events your endpoints can subscribe to.
        </p>
      </div>
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {WEBHOOK_EVENTS.map((group) => {
          const Icon = GROUP_ICONS[group.label] ?? Bell;
          const expanded = open[group.label] ?? true;
          return (
            <div
              key={group.label}
              className="rounded-xl border bg-card p-4"
            >
              <button
                type="button"
                onClick={() => toggle(group.label)}
                aria-expanded={expanded}
                className="flex w-full items-center gap-2 text-left"
              >
                <Icon className="size-4 shrink-0 text-muted-foreground" />
                <span className="flex-1 text-sm font-semibold">
                  {group.label}
                </span>
                <span className="rounded-full bg-background px-2 py-0.5 text-[11px] text-muted-foreground">
                  {group.events.length}
                </span>
                <ChevronDown
                  className={cn(
                    "size-4 shrink-0 text-muted-foreground transition-transform",
                    expanded && "rotate-180",
                  )}
                />
              </button>
              {expanded && (
                <ul className="mt-3 space-y-2">
                  {group.events.map((event) => (
                    <li
                      key={event.key}
                      className="flex items-baseline justify-between gap-2"
                    >
                      <code className="break-all font-mono text-[11px]">{event.key}</code>
                      <span className="shrink-0 text-xs text-muted-foreground">
                        {event.label}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          );
        })}
      </div>
    </section>
  );
}
