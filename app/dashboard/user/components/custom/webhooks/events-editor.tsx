"use client";

import { useState } from "react";
import { X } from "lucide-react";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

/**
 * Event-list chip editor for webhook triggers. Enter or comma adds;
 * backspace on empty removes the last chip. Duplicates + blanks collapse.
 */
export function EventsEditor({
  events,
  onChange,
  placeholder = "e.g. order.placed",
}: {
  events: string[];
  onChange: (next: string[]) => void;
  placeholder?: string;
}) {
  const [input, setInput] = useState("");

  const commit = () => {
    const parts = input
      .split(/[,\n]/)
      .map((p) => p.trim())
      .filter(Boolean);
    if (parts.length === 0) return;
    const next = [...events];
    for (const p of parts) {
      if (!next.includes(p)) next.push(p);
    }
    onChange(next);
    setInput("");
  };

  return (
    <div className="space-y-1.5">
      <div
        className={cn(
          "flex min-h-9 flex-wrap items-center gap-1.5 rounded-md border border-input bg-transparent px-2.5 py-1.5 text-sm",
          "focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/50",
        )}
      >
        {events.map((e) => (
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
        <Input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === ",") {
              e.preventDefault();
              commit();
            }
            if (e.key === "Backspace" && !input && events.length > 0) {
              onChange(events.slice(0, -1));
            }
          }}
          placeholder={events.length === 0 ? placeholder : "Add event…"}
          aria-label="Add event name"
          className="h-6 min-w-24 flex-1 border-0 bg-transparent p-0 font-mono text-xs shadow-none focus-visible:ring-0 dark:bg-transparent"
        />
      </div>
      <p className="text-xs text-muted-foreground">
        Trigger names from the caller. Enter or comma adds one; duplicates
        collapse.
      </p>
    </div>
  );
}
