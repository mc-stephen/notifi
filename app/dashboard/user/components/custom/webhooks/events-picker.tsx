"use client";

import { useState } from "react";
import { Check, X } from "lucide-react";
import { Input } from "@/components/ui/input";
import { WEBHOOK_EVENTS } from "@/lib/constants";
import { cn } from "@/lib/utils";

/** Word-prefix match so stray substrings (e.g. "art" in "start") don't hit. */
function matchesWords(haystack: string, q: string): boolean {
  if (!q) return true;
  return haystack
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .some((word) => word.startsWith(q));
}

/**
 * Catalog-driven event picker for webhook subscriptions — no typing. The
 * known events (the same WEBHOOK_EVENTS catalog the flow builder's
 * trigger presets come from) render by domain; each row toggles on/off.
 * Values inherited from the old free-text editor render as removable
 * chips so legacy assignments stay editable.
 */
export function EventsPicker({
  events,
  onChange,
}: {
  events: string[];
  onChange: (next: string[]) => void;
}) {
  const [query, setQuery] = useState("");

  const catalogKeys = new Set(
    WEBHOOK_EVENTS.flatMap((g) => g.events.map((e) => e.key)),
  );
  const custom = events.filter((e) => !catalogKeys.has(e));

  const q = query.toLowerCase();
  const groups = WEBHOOK_EVENTS.map((g) => ({
    label: g.label,
    events: g.events.filter((e) => matchesWords(`${g.label} ${e.label} ${e.key}`, q)),
  })).filter((g) => g.events.length > 0);

  const toggle = (key: string) =>
    onChange(
      events.includes(key) ? events.filter((e) => e !== key) : [...events, key],
    );

  return (
    <div className="space-y-2">
      <div className="flex min-h-9 items-center justify-between gap-2 px-0.5">
        <span className="text-xs text-muted-foreground">
          {events.length === 0
            ? "Nothing subscribed yet."
            : `${events.length} event${events.length === 1 ? "" : "s"} subscribed`}
        </span>
        {events.length > 0 && (
          <button
            type="button"
            onClick={() => onChange([])}
            className="text-xs font-medium text-muted-foreground underline-offset-2 hover:text-foreground hover:underline"
          >
            Clear all
          </button>
        )}
      </div>

      {custom.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {custom.map((e) => (
            <span
              key={e}
              className="flex items-center gap-1 rounded-md border border-border bg-muted px-1.5 py-0.5 font-mono text-[11px]"
            >
              {e}
              <button
                type="button"
                aria-label={`Remove ${e}`}
                className="rounded-sm text-muted-foreground hover:text-foreground"
                onClick={() => onChange(events.filter((x) => x !== e))}
              >
                <X className="size-3" />
              </button>
            </span>
          ))}
        </div>
      )}

      <div className="relative">
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search events…"
          aria-label="Search events"
          className="h-8 text-sm"
        />
      </div>

      <div className="max-h-60 space-y-1.5 overflow-y-auto rounded-lg border border-border p-1.5">
        {groups.length === 0 ? (
          <p className="px-2 py-6 text-center text-xs text-muted-foreground">
            No events match.
          </p>
        ) : (
          groups.map((group) => (
            <div key={group.label}>
              <p className="px-2.5 pb-0.5 pt-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                {group.label}
              </p>
              {group.events.map((event) => {
                const selected = events.includes(event.key);
                return (
                  <button
                    key={event.key}
                    type="button"
                    onClick={() => toggle(event.key)}
                    aria-pressed={selected}
                    className={cn(
                      "flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm transition-colors",
                      selected
                        ? "bg-primary/10 text-foreground"
                        : "text-muted-foreground hover:bg-muted hover:text-foreground",
                    )}
                  >
                    <span
                      className={cn(
                        "flex size-4 shrink-0 items-center justify-center rounded-full border",
                        selected
                          ? "border-primary bg-primary text-primary-foreground"
                          : "border-border text-transparent",
                      )}
                    >
                      <Check className="size-3" />
                    </span>
                    <span className="min-w-0 flex-1 truncate">{event.label}</span>
                    <span className="shrink-0 font-mono text-[10px] text-muted-foreground">
                      {event.key}
                    </span>
                  </button>
                );
              })}
            </div>
          ))
        )}
      </div>
      <p className="text-xs text-muted-foreground">
        Pick what this webhook receives — subscribed events fire the
        destination URL with the event&apos;s payload.
      </p>
    </div>
  );
}
