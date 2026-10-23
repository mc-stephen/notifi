"use client";

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
import type { ServerSchedule } from "@/hooks/use-schedules";

export type IntervalUnit = "minutes" | "hours" | "days";

export const UNIT_SECS: Record<IntervalUnit, number> = {
  minutes: 60,
  hours: 3600,
  days: 86400,
};

/** Day chips, Sunday-first to match the stored day numbers. */
export const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/** Editable form state shared by the create panel and the detail editor.
 * Content is not collected here — the schedule's flow owns the message
 * (a template node on the canvas). */
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

const KIND_CARDS = [
  { id: "once", label: "Once", blurb: "a single run" },
  { id: "recurring", label: "Repeats", blurb: "every N" },
  { id: "weekly", label: "Weekly", blurb: "days + a time" },
] as const;

/** Clicked (never rendered) quick stamps for one-shot schedules. */
const ONCE_CHIPS: { label: string; apply: (d: Date) => Date }[] = [
  { label: "In 1 hour", apply: (d) => new Date(d.getTime() + 3_600_000) },
  {
    label: "Tomorrow 9:00",
    apply: (d) => {
      d.setDate(d.getDate() + 1);
      d.setHours(9, 0, 0, 0);
      return d;
    },
  },
  {
    label: "Next Monday 9:00",
    apply: (d) => {
      do {
        d.setDate(d.getDate() + 1);
      } while (d.getDay() !== 1);
      d.setHours(9, 0, 0, 0);
      return d;
    },
  },
];

const REPEAT_PRESETS = [
  { label: "15 min", secs: 900 },
  { label: "1 hour", secs: 3600 },
  { label: "6 hours", secs: 21_600 },
  { label: "1 day", secs: 86_400 },
];

/** One plain-English sentence for the current timing state. */
export function describeTiming(v: ScheduleFormValue): string {
  if (v.kind === "once") {
    const iso = fromLocalInput(v.runAt);
    if (!iso) return "Runs once — pick a date and time";
    const local = new Date(iso);
    return `Runs once on ${local.toLocaleDateString(undefined, { month: "short", day: "numeric" })} at ${local.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}`;
  }
  if (v.kind === "recurring") {
    if (v.intervalAmount === 1) {
      const unit =
        v.intervalUnit === "minutes" ? "minute" : v.intervalUnit === "hours" ? "hour" : "day";
      return `Runs every ${unit}`;
    }
    return `Runs every ${v.intervalAmount} ${v.intervalUnit}`;
  }
  const dayNames = [...v.days].sort((a, b) => a - b).map((d) => WEEKDAYS[d]);
  const joined =
    dayNames.length === 5 &&
    ["Mon", "Tue", "Wed", "Thu", "Fri"].every((d) => dayNames.includes(d))
      ? "weekdays"
      : dayNames.length === 7
        ? "every day"
        : dayNames.join(", ");
  return `Runs ${joined} at ${v.runTime}`;
}

export function ScheduleFormFields({
  value,
  onChange,
}: {
  value: ScheduleFormValue;
  onChange: (next: ScheduleFormValue) => void;
}) {
  const set = (patch: Partial<ScheduleFormValue>) => onChange({ ...value, ...patch });
  const toggleDay = (day: number) =>
    set({
      days: value.days.includes(day)
        ? value.days.filter((d) => d !== day)
        : [...value.days, day].sort((a, b) => a - b),
    });
  const applyPreset = (secs: number) =>
    set(
      secs % 86400 === 0
        ? { intervalAmount: secs / 86400, intervalUnit: "days" }
        : secs % 3600 === 0
          ? { intervalAmount: secs / 3600, intervalUnit: "hours" }
          : { intervalAmount: secs / 60, intervalUnit: "minutes" },
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
          The schedule gets its own flow with this name — wire a template node
          into it in the builder; that flow owns the message.
        </p>
      </Field>

      <Field label="Timing">
        <div
          className="grid grid-cols-3 gap-1.5"
          role="group"
          aria-label="Timing cadence"
        >
          {KIND_CARDS.map((card) => {
            const selected = value.kind === card.id;
            return (
              <button
                key={card.id}
                type="button"
                onClick={() => set({ kind: card.id })}
                aria-pressed={selected}
                className={cn(
                  "rounded-lg border px-2 py-2 text-left transition-colors",
                  selected
                    ? "border-primary bg-primary text-primary-foreground"
                    : "border-border text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                <span className="block text-xs font-semibold">{card.label}</span>
                <span
                  className={cn(
                    "block text-[10px]",
                    selected ? "text-primary-foreground/80" : "text-muted-foreground/70",
                  )}
                >
                  {card.blurb}
                </span>
              </button>
            );
          })}
        </div>
      </Field>

      {value.kind === "once" ? (
        <Field label="Run at">
          <Input
            type="datetime-local"
            value={value.runAt}
            onChange={(e) => set({ runAt: e.target.value })}
            className="h-9 text-sm"
          />
          <div className="flex flex-wrap gap-1.5 pt-0.5">
            {ONCE_CHIPS.map((chip) => (
              <button
                key={chip.label}
                type="button"
                onClick={() => set({ runAt: toLocalInput(chip.apply(new Date()).toISOString()) })}
                className="h-7 rounded-md border px-2 text-xs font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
              >
                {chip.label}
              </button>
            ))}
          </div>
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
          <div className="flex flex-wrap gap-1.5 pt-0.5">
            {REPEAT_PRESETS.map((preset) => (
              <button
                key={preset.label}
                type="button"
                onClick={() => applyPreset(preset.secs)}
                className={cn(
                  "h-7 rounded-md border px-2 text-xs font-medium transition-colors",
                  value.intervalAmount * UNIT_SECS[value.intervalUnit] === preset.secs
                    ? "border-primary bg-primary text-primary-foreground"
                    : "border-border text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                {preset.label}
              </button>
            ))}
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

      <p className="rounded-lg bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
        {describeTiming(value)} · your local timezone ({localTimezone()})
      </p>
    </div>
  );
}
