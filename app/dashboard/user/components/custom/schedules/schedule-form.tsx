"use client";

import { useState } from "react";
import { Check, Search } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { useTemplates } from "@/hooks/use-templates";
import type { ServerSchedule } from "@/hooks/use-schedules";

export type IntervalUnit = "minutes" | "hours" | "days";

export const UNIT_SECS: Record<IntervalUnit, number> = {
  minutes: 60,
  hours: 3600,
  days: 86400,
};

/** Day chips, Sunday-first to match the stored day numbers. */
export const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/** Editable form state shared by the create panel and the detail editor. */
export type ScheduleFormValue = {
  name: string;
  kind: "once" | "recurring" | "weekly";
  /** `datetime-local` string (browser-local) for one-shot runs. */
  runAt: string;
  intervalAmount: number;
  intervalUnit: IntervalUnit;
  /** Selected weekdays (0 = Sunday .. 6 = Saturday). */
  days: number[];
  /** Weekly fire time ("HH:mm", browser-local). */
  runTime: string;
  templateId: string;
};

/** The creator's local IANA zone, cached per session. */
let cachedTimezone: string | null = null;
export function localTimezone(): string {
  if (!cachedTimezone) {
    try {
      cachedTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
    } catch {
      cachedTimezone = "UTC";
    }
  }
  return cachedTimezone;
}

/** `DateTime<Utc>` ISO → `datetime-local` input value. */
export function toLocalInput(iso: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** `datetime-local` input value → ISO string (or null when empty). */
export function fromLocalInput(local: string): string | null {
  if (!local.trim()) return null;
  const d = new Date(local);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

/** Split an interval in seconds into a round amount + unit for editing. */
export function splitInterval(totalSecs: number | null): {
  amount: number;
  unit: IntervalUnit;
} {
  const secs = totalSecs ?? 3600;
  if (secs % 86400 === 0) return { amount: secs / 86400, unit: "days" };
  if (secs % 3600 === 0) return { amount: secs / 3600, unit: "hours" };
  return { amount: Math.max(1, Math.round(secs / 60)), unit: "minutes" };
}

export function blankScheduleForm(): ScheduleFormValue {
  return {
    name: "",
    kind: "recurring",
    runAt: "",
    intervalAmount: 1,
    intervalUnit: "hours",
    days: [1],
    runTime: "09:00",
    templateId: "",
  };
}

export function scheduleToForm(s: ServerSchedule): ScheduleFormValue {
  const { amount, unit } = splitInterval(s.intervalSecs);
  return {
    name: s.name,
    kind: s.kind,
    runAt: toLocalInput(s.runAt),
    intervalAmount: amount,
    intervalUnit: unit,
    days: [...s.days].sort((a, b) => a - b),
    runTime: s.runTime ?? "09:00",
    templateId: s.templateId ?? "",
  };
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-1.5">
      <Label className="text-sm font-medium">{label}</Label>
      {children}
    </div>
  );
}

export function ScheduleFormFields({
  value,
  onChange,
}: {
  value: ScheduleFormValue;
  onChange: (next: ScheduleFormValue) => void;
}) {
  const { templates, loading: templatesLoading } = useTemplates();
  const [templateQuery, setTemplateQuery] = useState("");
  const set = (patch: Partial<ScheduleFormValue>) => onChange({ ...value, ...patch });
  const toggleDay = (day: number) =>
    set({
      days: value.days.includes(day)
        ? value.days.filter((d) => d !== day)
        : [...value.days, day].sort((a, b) => a - b),
    });

  const visibleTemplates = templates.filter((t) =>
    t.name.toLowerCase().includes(templateQuery.trim().toLowerCase()),
  );

  return (
    <div className="space-y-4">
      <Field label="Schedule name">
        <Input
          value={value.name}
          onChange={(e) => set({ name: e.target.value })}
          placeholder="e.g. Morning digest"
          className="h-9 text-sm"
        />
        <p className="text-xs text-muted-foreground">
          The schedule gets its own flow with this name — edit it from the
          Schedules page or the builder.
        </p>
      </Field>

      <Field label="Template">
        <div className="space-y-2">
          <div className="relative">
            <Search className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={templateQuery}
              onChange={(e) => setTemplateQuery(e.target.value)}
              placeholder="Search templates..."
              aria-label="Search templates"
              className="h-8 pl-8 text-sm"
            />
          </div>
          <div
            role="radiogroup"
            aria-label="Assign a template"
            className="max-h-44 space-y-1 overflow-y-auto rounded-lg border border-border p-1"
          >
            {templatesLoading ? (
              <p className="px-2 py-4 text-center text-xs text-muted-foreground">
                Loading templates…
              </p>
            ) : visibleTemplates.length === 0 ? (
              <p className="px-2 py-4 text-center text-xs text-muted-foreground">
                {templates.length === 0
                  ? "No templates yet — create one to get started."
                  : "No templates match."}
              </p>
            ) : (
              visibleTemplates.map((t) => {
                const selected = value.templateId === t.id;
                return (
                  <button
                    key={t.id}
                    type="button"
                    role="radio"
                    aria-checked={selected}
                    onClick={() => set({ templateId: t.id })}
                    className={cn(
                      "flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm transition-colors hover:bg-muted",
                      selected && "bg-muted",
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
                    <span className="min-w-0 flex-1 truncate font-medium">{t.name}</span>
                  </button>
                );
              })
            )}
          </div>
        </div>
        <p className="text-xs text-muted-foreground">
          Create the template on the Templates page first — the schedule
          renders it on every run.
        </p>
      </Field>

      <Field label="Timing">
        <Select
          value={value.kind}
          onValueChange={(v) => v && set({ kind: v as ScheduleFormValue["kind"] })}
        >
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="once">Run once</SelectItem>
            <SelectItem value="recurring">Repeat</SelectItem>
            <SelectItem value="weekly">Weekly</SelectItem>
          </SelectContent>
        </Select>
      </Field>

      {value.kind === "once" ? (
        <Field label="Run at">
          <Input
            type="datetime-local"
            value={value.runAt}
            onChange={(e) => set({ runAt: e.target.value })}
            className="h-9 text-sm"
          />
        </Field>
      ) : value.kind === "recurring" ? (
        <Field label="Repeat every">
          <div className="flex items-center gap-2">
            <Input
              type="number"
              min={1}
              step={1}
              value={value.intervalAmount}
              onChange={(e) => {
                const n = Math.floor(Number(e.target.value));
                if (Number.isFinite(n) && n >= 1) set({ intervalAmount: n });
              }}
              className="h-9 w-24 text-sm"
            />
            <Select
              value={value.intervalUnit}
              onValueChange={(v) => v && set({ intervalUnit: v as IntervalUnit })}
            >
              <SelectTrigger className="flex-1">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="minutes">Minutes</SelectItem>
                <SelectItem value="hours">Hours</SelectItem>
                <SelectItem value="days">Days</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <p className="text-xs text-muted-foreground">Minimum interval is 1 minute.</p>
        </Field>
      ) : (
        <Field label="Run on">
          <div className="flex flex-wrap gap-1.5">
            {WEEKDAYS.map((label, day) => {
              const selected = value.days.includes(day);
              return (
                <button
                  key={label}
                  type="button"
                  onClick={() => toggleDay(day)}
                  aria-pressed={selected}
                  className={cn(
                    "h-8 min-w-9 rounded-md border px-2 text-xs font-medium transition-colors",
                    selected
                      ? "border-primary bg-primary text-primary-foreground"
                      : "border-border text-muted-foreground hover:bg-muted hover:text-foreground",
                  )}
                >
                  {label}
                </button>
              );
            })}
          </div>
          <div className="flex items-center gap-2 pt-1">
            <span className="text-xs text-muted-foreground">at</span>
            <Input
              type="time"
              value={value.runTime}
              onChange={(e) => set({ runTime: e.target.value })}
              className="h-9 w-32 text-sm"
            />
          </div>
        </Field>
      )}

      <p className="text-xs text-muted-foreground">
        Times use your local timezone ({localTimezone()}).
      </p>
    </div>
  );
}
