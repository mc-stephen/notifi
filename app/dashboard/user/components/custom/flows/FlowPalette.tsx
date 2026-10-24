"use client";

import { useState } from "react";
import { ChevronDown, FileText, Hash, Inbox, KeyRound, Mail, Megaphone, MessageSquare, OctagonX, Phone, Plug, Search, Send, Smartphone, Sparkles, Split, Globe, UserRound, Users, Webhook, Zap } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import { WEBHOOK_EVENTS } from "@/lib/constants";
import { useProviderRegistry } from "@/hooks/use-provider-registry";
import type { FlowNodeKind } from "./flow-types";

export const DRAG_MIME = "application/notifi-flow-node";
/** Trigger group carried alongside the node kind on drag. */
export const TRIGGER_PRESET_MIME = "application/notifi-trigger-preset";
/** Provider channel carried alongside the node kind on drag. */
export const PROVIDER_CHANNEL_MIME = "application/notifi-provider-channel";

export type TriggerPreset = string;

const CHANNEL_ICONS: Record<string, typeof Zap> = {
  email: Mail,
  sms: MessageSquare,
  push: Smartphone,
  chat: Hash,
  voice: Phone,
  whatsapp: Phone,
  in_app: Inbox,
  webhook: Webhook,
};

const GROUP_ICONS: Record<string, typeof Zap> = {
  Notifications: Zap,
  Campaigns: Megaphone,
  Providers: Plug,
  "API Keys": KeyRound,
  Teams: Users,
  Recipients: UserRound,
  Templates: FileText,
};

const ITEMS: { kind: FlowNodeKind; title: string; blurb: string; icon: typeof Zap }[] = [
  { kind: "template", title: "Template", blurb: "Render a saved template's content", icon: FileText },
  { kind: "http-request", title: "HTTP Request", blurb: "Call any URL mid-flow", icon: Globe },
  { kind: "condition", title: "Condition", blurb: "Branch on recipient or message rules", icon: Split },
  { kind: "ai-transform", title: "AI transform", blurb: "Rewrite content with a model", icon: Sparkles },
  { kind: "stop", title: "Stop", blurb: "End this branch", icon: OctagonX },
];

const TRIGGER_PARTS: { preset: TriggerPreset; title: string; blurb: string; icon: typeof Zap }[] = [
  ...WEBHOOK_EVENTS.map((g) => {
    const singular = g.label.replace(/s$/, "");
    return {
      preset: g.label,
      title: singular,
      blurb: `Start on ${singular.toLowerCase()} events`,
      icon: GROUP_ICONS[g.label] ?? Zap,
    };
  }),
];

/** Word-prefix match so stray substrings (e.g. "art" in "start") don't hit. */
function matchesWords(haystack: string, q: string): boolean {
  if (!q) return true;
  return haystack
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .some((word) => word.startsWith(q));
}

function DraggableRow({
  title,
  blurb,
  icon: Icon,
  kind,
  preset,
  channel,
  indent,
}: {
  title: string;
  blurb: string;
  icon: typeof Zap;
  kind: FlowNodeKind;
  preset?: TriggerPreset;
  channel?: string;
  indent?: boolean;
}) {
  return (
    <div
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData(DRAG_MIME, kind);
        if (preset) e.dataTransfer.setData(TRIGGER_PRESET_MIME, preset);
        if (channel) e.dataTransfer.setData(PROVIDER_CHANNEL_MIME, channel);
        e.dataTransfer.effectAllowed = "move";
      }}
      className={cn(
        "flex cursor-grab items-center gap-2.5 rounded-lg border border-transparent p-2.5",
        "transition-colors hover:border-border hover:bg-muted active:cursor-grabbing dark:hover:border-white/10 dark:hover:bg-white/5",
        indent && "ml-4",
      )}
    >
      <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
        <Icon className="size-4" />
      </span>
      <span className="min-w-0">
        <span className="block truncate text-sm font-medium text-foreground dark:text-zinc-100">{title}</span>
        <span className="block truncate text-xs text-muted-foreground dark:text-zinc-500">{blurb}</span>
      </span>
    </div>
  );
}

export function FlowPalette({ hideTriggers = false }: { hideTriggers?: boolean }) {
  const [query, setQuery] = useState("");
  const [triggersOpen, setTriggersOpen] = useState(true);
  const [providersOpen, setProvidersOpen] = useState(true);
  const { registry, loading: providersLoading, error: providersError } = useProviderRegistry();
  const providerChannels = registry?.channels ?? [];

  const q = query.toLowerCase();
  const visible = ITEMS.filter((item) =>
    `${item.title} ${item.blurb}`.toLowerCase().includes(q),
  );
  const triggerMatch =
    matchesWords("trigger start the flow", q) ||
    TRIGGER_PARTS.some((p) => matchesWords(`${p.title} ${p.blurb}`, q));
  const visibleParts = TRIGGER_PARTS.filter((p) =>
    matchesWords(`${p.title} ${p.blurb}`, q),
  );
  const providerMatch =
    matchesWords("send via provider deliver", q) ||
    providerChannels.some((c) =>
      matchesWords(`${c.channel_name} send via providers`, q),
    );
  const visibleChannels = providerChannels.filter((c) =>
    matchesWords(`${c.channel_name} send via providers`, q),
  );

  return (
    <div className="flex w-60 shrink-0 flex-col self-stretch border-r border-border bg-card dark:border-white/10 dark:bg-[#0b0b0e]">
      <div className="border-b border-border p-3 dark:border-white/10">
        <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground dark:text-zinc-500">
          Components
        </p>
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground dark:text-zinc-500" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search components"
            className="h-8 border-input bg-popover pl-8 text-sm text-foreground placeholder:text-muted-foreground dark:border-white/10 dark:bg-white/5 dark:text-zinc-100 dark:placeholder:text-zinc-500"
          />
        </div>
      </div>
      <div className="flex-1 space-y-1 overflow-y-auto p-2">
        {!hideTriggers && (!query || triggerMatch) && (
          <div>
            <button
              type="button"
              onClick={() => setTriggersOpen((o) => !o)}
              aria-expanded={triggersOpen}
              className={cn(
                "flex w-full cursor-pointer items-center gap-2.5 rounded-lg border border-transparent p-2.5",
                "transition-colors hover:border-border hover:bg-muted dark:hover:border-white/10 dark:hover:bg-white/5",
              )}
            >
              <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <Zap className="size-4" />
              </span>
              <span className="min-w-0 flex-1 text-left">
                <span className="block truncate text-sm font-medium text-foreground dark:text-zinc-100">Trigger</span>
                <span className="block truncate text-xs text-muted-foreground dark:text-zinc-500">Start the flow</span>
              </span>
              <ChevronDown
                className={cn("size-3.5 shrink-0 text-muted-foreground transition-transform dark:text-zinc-500", !triggersOpen && "-rotate-90")}
              />
            </button>
            {triggersOpen &&
              (query ? visibleParts : TRIGGER_PARTS).map((p) => (
                <DraggableRow
                  key={p.preset}
                  title={p.title}
                  blurb={p.blurb}
                  icon={p.icon}
                  kind="trigger"
                  preset={p.preset}
                  indent
                />
              ))}
          </div>
        )}
        {(!hideTriggers || visible.length > 0 || providerMatch) &&
          visible.map((item) => (
          <DraggableRow
            key={item.kind}
            title={item.title}
            blurb={item.blurb}
            icon={item.icon}
            kind={item.kind}
          />
        ))}
        {(!query || providerMatch) && (
          <div>
            <button
              type="button"
              onClick={() => setProvidersOpen((o) => !o)}
              aria-expanded={providersOpen}
              className={cn(
                "flex w-full cursor-pointer items-center gap-2.5 rounded-lg border border-transparent p-2.5",
                "transition-colors hover:border-border hover:bg-muted dark:hover:border-white/10 dark:hover:bg-white/5",
              )}
            >
              <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                <Send className="size-4" />
              </span>
              <span className="min-w-0 flex-1 text-left">
                <span className="block truncate text-sm font-medium text-foreground dark:text-zinc-100">Send via provider</span>
                <span className="block truncate text-xs text-muted-foreground dark:text-zinc-500">Deliver with priority + fallback</span>
              </span>
              <ChevronDown
                className={cn("size-3.5 shrink-0 text-muted-foreground transition-transform dark:text-zinc-500", !providersOpen && "-rotate-90")}
              />
            </button>
            {providersOpen && (
              providersLoading ? (
                <div className="ml-4 space-y-1">
                  {[0, 1, 2].map((i) => (
                    <Skeleton key={i} className="h-[52px] rounded-lg bg-muted dark:bg-white/5" />
                  ))}
                </div>
              ) : providersError || providerChannels.length === 0 ? (
                <p className="ml-4 px-2 py-3 text-xs text-zinc-500">
                  {providersError ? "Couldn't load channels." : "No channels configured."}
                </p>
              ) : (
                (query ? visibleChannels : providerChannels).map((c) => {
                  const Icon = CHANNEL_ICONS[c.channel_id] ?? Send;
                  const count = c.providers.length;
                  return (
                    <DraggableRow
                      key={c.channel_id}
                      title={c.channel_name}
                      blurb={count === 1 ? "1 provider" : `${count} providers`}
                      icon={Icon}
                      kind="provider"
                      channel={c.channel_id}
                      indent
                    />
                  );
                })
              )
            )}
          </div>
        )}
        {visible.length === 0 &&
          !providerMatch &&
          (hideTriggers || !triggerMatch) && (
          <p className="px-2 py-6 text-center text-sm text-zinc-500">
            No components match.
          </p>
        )}
      </div>
    </div>
  );
}
