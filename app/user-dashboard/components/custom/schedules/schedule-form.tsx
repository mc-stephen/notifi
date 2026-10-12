"use client";

import { useState } from "react";
import { Check, Loader2, Search, Trash2 } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { handlesSendEvent } from "@/hooks/use-flows";
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
  flowId: string;
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
    flowId: "",
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
    flowId: s.flowId,
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
  flowOptions,
  onCreateFlow,
  creatingFlow,
  lockedFlowId,
  onDeleteLockedFlow,
  deletingLocked,
}: {
  value: ScheduleFormValue;
  onChange: (next: ScheduleFormValue) => void;
  flowOptions: { id: string; name: string; definition?: unknown }[];
  onCreateFlow?: () => void;
  creatingFlow?: boolean;
  /** A just-created flow bound to this schedule: picker locks onto it. */
  lockedFlowId?: string | null;
  onDeleteLockedFlow?: () => void;
  deletingLocked?: boolean;
}) {
  const { templates, loading: templatesLoading } = useTemplates();
  const [flowQuery, setFlowQuery] = useState("");
  const set = (patch: Partial<ScheduleFormValue>) => onChange({ ...value, ...patch });
  const toggleDay = (day: number) =>
    set({
      days: value.days.includes(day)
        ? value.days.filter((d) => d !== day)
        : [...value.days, day].sort((a, b) => a - b),
    });

  const lockedFlow = lockedFlowId
    ? (flowOptions.find((f) => f.id === lockedFlowId) ?? {
        id: lockedFlowId,
        name: "New flow",
      })
    : null;
  const visibleFlows = flowOptions.filter((f) =>
    f.name.toLowerCase().includes(flowQuery.trim().toLowerCase()),
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
      </Field>

      <Field label="Flow">
        {lockedFlow ? (
          <div>
            <div className="flex items-center gap-2 rounded-lg border border-primary/40 bg-primary/5 px-2.5 py-2">
              <Check className="size-3.5 shrink-0 text-primary" />
              <span className="min-w-0 flex-1 truncate text-sm font-medium">
                {lockedFlow.name}
              </span>
              <Badge variant="secondary" className="shrink-0 text-[10px] font-medium">
                New
              </Badge>
              {onDeleteLockedFlow && (
                <button
                  type="button"
                  onClick={onDeleteLockedFlow}
                  disabled={deletingLocked}
                  title="Delete this flow"
                  aria-label={`Delete ${lockedFlow.name}`}
                  className="shrink-0 rounded-md p-1 text-destructive hover:bg-destructive/10 disabled:opacity-50"
                >
                  <Trash2 className="size-3.5" />
                </button>
              )}
            </div>
            <p className="mt-1 text-xs text-muted-foreground">
              Bound to this schedule — delete it here to pick another flow.
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <div className="relative flex-1">
                <Search className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
                <Input
                  value={flowQuery}
                  onChange={(e) => setFlowQuery(e.target.value)}
                  placeholder="Search flows..."
                  aria-label="Search flows"
                  className="h-8 pl-8 text-sm"
                />
              </div>
              {onCreateFlow && (
                <button
                  type="button"
                  onClick={onCreateFlow}
                  disabled={creatingFlow}
                  className="flex h-8 shrink-0 items-center gap-1.5 rounded-md border border-border px-2.5 text-xs font-medium text-muted-foreground hover:bg-muted hover:text-foreground disabled:opacity-50"
                >
                  {creatingFlow && <Loader2 className="size-3 animate-spin" />}
                  {creatingFlow ? "Creating…" : "New flow"}
                </button>
              )}
            </div>
            <div
              role="radiogroup"
              aria-label="Assign a flow"
              className="max-h-44 space-y-1 overflow-y-auto rounded-lg border border-border p-1"
            >
              {visibleFlows.length === 0 ? (
                <p className="px-2 py-4 text-center text-xs text-muted-foreground">
                  {flowOptions.length === 0
                    ? "No flows yet — create one to get started."
                    : "No flows match."}
                </p>
              ) : (
                visibleFlows.map((f) => {
                  const selected = value.flowId === f.id;
                  return (
                    <button
                      key={f.id}
                      type="button"
                      role="radio"
                      aria-checked={selected}
                      onClick={() => set({ flowId: f.id })}
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
                      <span className="min-w-0 flex-1 truncate font-medium">{f.name}</span>
                      {handlesSendEvent(f.definition) && (
                        <Badge variant="secondary" className="shrink-0 text-[10px] font-medium">
                          Handles sends
                        </Badge>
                      )}
                    </button>
                  );
                })
              )}
            </div>
          </div>
        )}
        <p className="text-xs text-muted-foreground">
          Assigned flows leave the Flows page and are managed here.
        </p>
      </Field>

      <Field label="Template">
        <Select
          value={value.templateId || undefined}
          onValueChange={(v) => v && set({ templateId: v })}
        >
          <SelectTrigger>
            <SelectValue
              placeholder={templatesLoading ? "Loading templates…" : "Select a template"}
            />
          </SelectTrigger>
          <SelectContent>
            {templates.map((t) => (
              <SelectItem key={t.id} value={t.id}>
                {t.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
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
