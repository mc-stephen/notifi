"use client";

import { useState } from "react";
import { ArrowLeft, FileText, Hash, Inbox, KeyRound, Mail, Megaphone, MessageSquare, OctagonX, Phone, Plug, Search, Send, Smartphone, Sparkles, Split, Globe, UserRound, Users, Webhook, Workflow, Zap } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import { WEBHOOK_EVENTS } from "@/lib/constants";
import { useProviderRegistry } from "@/hooks/use-provider-registry";
import type { FlowNodeKind } from "./flow-types";

export const DRAG_MIME = "application/notifi-flow-node";
/** Trigger group carried alongside the node kind on drag. */
export const TRIGGER_PRESET_MIME = "application/notifi-trigger-preset";
/** Trigger kind (webhook) carried alongside the node kind on drag. */
export const TRIGGER_KIND_MIME = "application/notifi-trigger-kind";
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

const TRIGGER_PARTS: {
  preset?: TriggerPreset;
  triggerKind?: "webhook";
  title: string;
  blurb: string;
  icon: typeof Zap;
}[] = [
  ...WEBHOOK_EVENTS.map((g) => {
    const singular = g.label.replace(/s$/, "");
    return {
      preset: g.label,
      title: singular,
      blurb: `Start on ${singular.toLowerCase()} events`,
      icon: GROUP_ICONS[g.label] ?? Zap,
    };
  }),
  {
    triggerKind: "webhook",
    title: "Inbound webhook",
    blurb: "Start when something POSTs to this flow's URL",
    icon: Webhook,
  },
];

/** The panel's drill-down categories: home shows one tile per category,
 * picking one swaps the body to that category's children only. */
type CategoryId = "trigger" | "actions" | "providers";

const CATEGORIES: { id: CategoryId; title: string; icon: typeof Zap }[] = [
  { id: "trigger", title: "Trigger", icon: Zap },
  { id: "actions", title: "Actions", icon: Workflow },
  { id: "providers", title: "Send via provider", icon: Send },
];

const CATEGORY_TITLES: Record<CategoryId, string> = {
  trigger: "Trigger",
  actions: "Actions",
  providers: "Send via provider",
};

/** A draggable palette row's full spec — shared by the drill-down body
 * and the flat search results. */
type RowSpec = {
  key: string;
  title: string;
  blurb: string;
  icon: typeof Zap;
  kind: FlowNodeKind;
  preset?: TriggerPreset;
  triggerKind?: "webhook";
  channel?: string;
};

function plural(n: number, word: string): string {
  return n === 1 ? `1 ${word}` : `${n} ${word}s`;
}

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
  triggerKind,
  channel,
  categoryLabel,
}: {
  title: string;
  blurb: string;
  icon: typeof Zap;
  kind: FlowNodeKind;
  preset?: TriggerPreset;
  triggerKind?: "webhook";
  channel?: string;
  /** Search results carry their category for context; drill-down rows don't. */
  categoryLabel?: string;
}) {
  return (
    <div
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData(DRAG_MIME, kind);
        if (preset) e.dataTransfer.setData(TRIGGER_PRESET_MIME, preset);
        if (triggerKind) e.dataTransfer.setData(TRIGGER_KIND_MIME, triggerKind);
        if (channel) e.dataTransfer.setData(PROVIDER_CHANNEL_MIME, channel);
        e.dataTransfer.effectAllowed = "move";
      }}
      className={cn(
        "flex cursor-grab items-center gap-2.5 rounded-lg border border-transparent p-2.5",
        "transition-colors hover:border-border hover:bg-muted active:cursor-grabbing dark:hover:border-white/10 dark:hover:bg-white/5",
      )}
    >
      <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
        <Icon className="size-4" />
      </span>
      <span className="min-w-0">
        <span className="block truncate text-sm font-medium text-foreground dark:text-zinc-100">{title}</span>
        <span className="block truncate text-xs text-muted-foreground dark:text-zinc-500">
          {categoryLabel ? `${categoryLabel} · ${blurb}` : blurb}
        </span>
      </span>
    </div>
  );
}

export function FlowPalette({ hideTriggers = false }: { hideTriggers?: boolean }) {
  const [query, setQuery] = useState("");
  const [view, setView] = useState<{ type: "home" } | { type: "category"; id: CategoryId }>({
    type: "home",
  });
  const { registry, loading: providersLoading, error: providersError } = useProviderRegistry();
  const providerChannels = registry?.channels ?? [];

  const q = query.toLowerCase();

  const triggerRows: RowSpec[] =
    hideTriggers ? [] : TRIGGER_PARTS.map((p) => ({
      key: p.triggerKind ?? p.preset ?? p.title,
      title: p.title,
      blurb: p.blurb,
      icon: p.icon,
      kind: "trigger",
      preset: p.preset,
      triggerKind: p.triggerKind,
    }));
  const actionRows: RowSpec[] = ITEMS.map((item) => ({
    key: item.kind,
    title: item.title,
    blurb: item.blurb,
    icon: item.icon,
    kind: item.kind,
  }));
  const providerRows: RowSpec[] = providerChannels.map((c) => ({
    key: c.channel_id,
    title: c.channel_name,
    blurb: plural(c.providers.length, "provider"),
    icon: CHANNEL_ICONS[c.channel_id] ?? Send,
    kind: "provider",
    channel: c.channel_id,
  }));

  // The category counts the tiles show.
  const counts: Record<CategoryId, string> = {
    trigger: plural(triggerRows.length, "item"),
    actions: plural(actionRows.length, "item"),
    providers: providersLoading ? "…" : plural(providerRows.length, "channel"),
  };

  // Search outranks the drill-down: typing anywhere flips the panel to a
  // flat results list across every category (each row names its own).
  const providerMatch = matchesWords(CATEGORY_TITLES.providers, q);
  const searchIndex: (RowSpec & { category: string })[] = [
    ...triggerRows.map((r) => ({ ...r, category: CATEGORY_TITLES.trigger })),
    ...actionRows.map((r) => ({ ...r, category: CATEGORY_TITLES.actions })),
    ...providerRows.map((r) => ({ ...r, category: CATEGORY_TITLES.providers })),
  ];
  const results = query
    ? searchIndex.filter(({ category, ...row }) =>
        matchesWords(`${category} ${row.title} ${row.blurb}`, q),
      )
    : [];

  const tiles = CATEGORIES.filter((c) => c.id !== "trigger" || !hideTriggers);
  const openCategory = !query && view.type === "category" ? view.id : null;

  return (
    <div className="flex w-60 shrink-0 flex-col self-stretch border-r border-border bg-card dark:border-white/10 dark:bg-[#0b0b0e]">
      <div className="border-b border-border p-3 dark:border-white/10">
        {openCategory ? (
          <button
            type="button"
            onClick={() => setView({ type: "home" })}
            className={cn(
              "mb-1 flex w-full cursor-pointer items-center gap-2 rounded-lg border border-transparent p-1.5",
              "transition-colors hover:border-border hover:bg-muted dark:hover:border-white/10 dark:hover:bg-white/5",
            )}
          >
            <ArrowLeft className="size-4 shrink-0 text-muted-foreground dark:text-zinc-500" />
            <span className="min-w-0 flex-1 truncate text-left text-sm font-medium text-foreground dark:text-zinc-100">
              {CATEGORY_TITLES[openCategory]}
            </span>
            <span className="shrink-0 text-xs text-muted-foreground dark:text-zinc-500">
              {counts[openCategory]}
            </span>
          </button>
        ) : (
          <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground dark:text-zinc-500">
            Components
          </p>
        )}
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground dark:text-zinc-500" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== "Escape") return;
              e.stopPropagation();
              if (query) setQuery("");
              else setView({ type: "home" });
            }}
            placeholder="Search components"
            className="h-8 border-input bg-popover pl-8 text-sm text-foreground placeholder:text-muted-foreground dark:border-white/10 dark:bg-white/5 dark:text-zinc-100 dark:placeholder:text-zinc-500"
          />
        </div>
      </div>
      <div className="flex-1 space-y-1 overflow-y-auto p-2">
        {/* Search results — flat across all categories. */}
        {query &&
          results.map(({ category, key, ...row }) => (
            <DraggableRow key={key} {...row} categoryLabel={category} />
          ))}
        {query && providersLoading && providerMatch && (
          <div className="space-y-1">
            {[0, 1, 2].map((i) => (
              <Skeleton key={i} className="h-[52px] rounded-lg bg-muted dark:bg-white/5" />
            ))}
          </div>
        )}
        {query &&
          results.length === 0 &&
          !(providersLoading && providerMatch) && (
          <p className="px-2 py-6 text-center text-sm text-zinc-500">
            No components match.
          </p>
        )}
        {/* Home — one plain row per category; picking one drills in. */}
        {!query && !openCategory && (
          <>
            {tiles.map((cat) => (
              <button
                key={cat.id}
                type="button"
                onClick={() => setView({ type: "category", id: cat.id })}
                className={cn(
                  "flex w-full cursor-pointer items-center gap-2.5 rounded-lg border border-transparent p-2.5",
                  "transition-colors hover:border-border hover:bg-muted dark:hover:border-white/10 dark:hover:bg-white/5",
                )}
              >
                <span className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <cat.icon className="size-4" />
                </span>
                <span className="min-w-0 flex-1 truncate text-left text-sm font-medium text-foreground dark:text-zinc-100">
                  {cat.title}
                </span>
                <span className="shrink-0 text-xs text-muted-foreground dark:text-zinc-500">
                  {counts[cat.id]}
                </span>
              </button>
            ))}
          </>
        )}
        {/* Drill-down — only the open category's children. */}
        {openCategory === "trigger" &&
          triggerRows.map(({ key, ...row }) => <DraggableRow key={key} {...row} />)}
        {openCategory === "actions" &&
          actionRows.map(({ key, ...row }) => <DraggableRow key={key} {...row} />)}
        {openCategory === "providers" &&
          (providersLoading ? (
            <div className="space-y-1">
              {[0, 1, 2].map((i) => (
                <Skeleton key={i} className="h-[52px] rounded-lg bg-muted dark:bg-white/5" />
              ))}
            </div>
          ) : providersError || providerChannels.length === 0 ? (
            <p className="px-2 py-3 text-xs text-zinc-500">
              {providersError ? "Couldn't load channels." : "No channels configured."}
            </p>
          ) : (
            providerRows.map(({ key, ...row }) => <DraggableRow key={key} {...row} />)
          ))}
      </div>
    </div>
  );
}
