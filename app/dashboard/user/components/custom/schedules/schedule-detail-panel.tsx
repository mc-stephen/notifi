"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { format, formatDistanceToNow } from "date-fns";
import {
  Archive,
  ArrowUpRight,
  Pause,
  Play,
  Rocket,
  Trash2,
  Undo2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { DetailPanel } from "@/components/custom/detail-panel";
import {
  UNIT_SECS,
  fromLocalInput,
  localTimezone,
  scheduleToForm,
  ScheduleFormFields,
  type ScheduleFormValue,
} from "./schedule-form";
import {
  patchSchedule,
  runScheduleNow,
  type ServerSchedule,
} from "@/hooks/use-schedules";
import { toast } from "sonner";

function fmtTime(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "—" : format(d, "MMM d, HH:mm");
}

/**
 * Right-side schedule editor: run metadata, full field form, and an
 * Open-flow button deep-linking to the schedule's own flow builder.
 */
export function ScheduleDetailPanel({
  schedule,
  projectId,
  onClose,
  onChanged,
  onDeleteRequest,
}: {
  schedule: ServerSchedule;
  projectId: string;
  onClose: () => void;
  /** Refresh the page list after a mutation (receives the updated row). */
  onChanged: (next: ServerSchedule) => void;
  onDeleteRequest: (schedule: ServerSchedule) => void;
}) {
  const router = useRouter();
  const [form, setForm] = useState<ScheduleFormValue>(() => scheduleToForm(schedule));
  const [saving, setSaving] = useState(false);
  const [running, setRunning] = useState(false);
  const [toggling, setToggling] = useState(false);
  const [archiving, setArchiving] = useState(false);

  const archived = schedule.archivedAt != null;

  const close = () => onClose();

  const handleSave = async () => {
    setSaving(true);
    try {
      // The form edits name + timing only; the stored message fields
      // round-trip unchanged (content lives in the flow's template node).
      const updated = await patchSchedule(projectId, schedule.id, {
        name: form.name,
        kind: form.kind,
        runAt: form.kind === "once" ? fromLocalInput(form.runAt) : null,
        intervalSecs:
          form.kind === "recurring"
            ? form.intervalAmount * UNIT_SECS[form.intervalUnit]
            : null,
        days: form.kind === "weekly" ? form.days : [],
        runTime: form.kind === "weekly" ? form.runTime : null,
        timezone: localTimezone(),
        channel: schedule.channel,
        title: schedule.title,
        body: schedule.body,
        recipients: schedule.recipients,
      });
      toast.success("Schedule updated.");
      onChanged(updated);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't update schedule.");
    } finally {
      setSaving(false);
    }
  };

  const handleRunNow = async () => {
    setRunning(true);
    try {
      const next = await runScheduleNow(projectId, schedule.id);
      toast.success(
        next.lastStatus === "completed" ? "Schedule ran successfully." : "Run finished with errors.",
      );
      onChanged(next);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't run schedule.");
    } finally {
      setRunning(false);
    }
  };

  const handleToggleActive = async () => {
    setToggling(true);
    try {
      const updated = await patchSchedule(projectId, schedule.id, {
        active: !schedule.active,
      });
      toast.success(schedule.active ? "Schedule paused." : "Schedule resumed.");
      onChanged(updated);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't update schedule.");
    } finally {
      setToggling(false);
    }
  };

  const handleToggleArchived = async () => {
    setArchiving(true);
    try {
      const updated = await patchSchedule(projectId, schedule.id, {
        archived: !archived,
      });
      toast.success(archived ? "Schedule restored." : "Schedule archived.");
      onChanged(updated);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't update schedule.");
    } finally {
      setArchiving(false);
    }
  };

  return (
    <DetailPanel
      onClose={close}
      header={
        <div>
          <div className="flex items-center gap-2">
            <h3 className="truncate font-semibold text-foreground">{schedule.name}</h3>
            {archived ? (
              <Badge className="border-warning/20 bg-warning/15 text-[10px] font-medium text-warning">
                Archived
              </Badge>
            ) : (
              <Badge
                variant="secondary"
                className={
                  schedule.active
                    ? "border-success/20 bg-success/15 text-[10px] font-medium text-success"
                    : "text-[10px] font-medium"
                }
              >
                {schedule.active ? "Active" : "Paused"}
              </Badge>
            )}
          </div>
          <p className="mt-0.5 truncate text-xs text-muted-foreground">
            {schedule.timing} · {schedule.flowName || "No flow"}
          </p>
        </div>
      }
    >
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="space-y-4 px-5 py-4">
          <dl className="grid grid-cols-2 gap-x-4 gap-y-3 rounded-lg border border-border bg-card p-3 text-sm">
            <div>
              <dt className="text-xs text-muted-foreground">Next run</dt>
              <dd className="mt-0.5 font-medium">
                {archived ? (
                  <span className="font-normal text-muted-foreground">Paused while archived</span>
                ) : schedule.nextRunAt ? (
                  <>
                    {fmtTime(schedule.nextRunAt)}{" "}
                    <span className="font-normal text-muted-foreground">
                      ({formatDistanceToNow(new Date(schedule.nextRunAt), { addSuffix: true })})
                    </span>
                  </>
                ) : (
                  "—"
                )}
              </dd>
            </div>
            <div>
              <dt className="text-xs text-muted-foreground">Total runs</dt>
              <dd className="mt-0.5 font-medium">{schedule.runCount}</dd>
            </div>
            <div className="col-span-2">
              <dt className="text-xs text-muted-foreground">Template</dt>
              <dd className="mt-0.5 font-medium">
                {schedule.templateName ?? (
                  <span className="font-normal text-muted-foreground">—</span>
                )}
              </dd>
            </div>
            <div className="col-span-2">
              <dt className="text-xs text-muted-foreground">Last run</dt>
              <dd className="mt-0.5">
                {schedule.lastRunAt ? (
                  <>
                    <span className="font-medium">{fmtTime(schedule.lastRunAt)}</span>{" "}
                    {schedule.lastStatus === "completed" ? (
                      <span className="text-xs text-success">completed</span>
                    ) : schedule.lastStatus ? (
                      <span className="text-xs text-destructive">
                        failed{schedule.lastError ? ` — ${schedule.lastError}` : ""}
                      </span>
                    ) : null}
                  </>
                ) : (
                  <span className="text-muted-foreground">Never</span>
                )}
              </dd>
            </div>
          </dl>

          <Button
            variant="outline"
            size="sm"
            className="w-full gap-1.5"
            onClick={() => router.push(`/flows/${schedule.flowId}`)}
          >
            Open flow builder <ArrowUpRight className="size-3.5" />
            <span className="sr-only">Opens {schedule.flowName || "the flow"} in the visual builder</span>
          </Button>

          <Separator />

          <ScheduleFormFields value={form} onChange={setForm} />

          <Button size="sm" className="w-full" onClick={() => void handleSave()} disabled={saving}>
            {saving ? "Saving…" : "Save changes"}
          </Button>
        </div>
      </div>

      <div className="flex gap-2 border-t border-border p-4">
        <Button
          variant="outline"
          size="sm"
          className="flex-1 gap-1.5"
          onClick={() => void handleRunNow()}
          disabled={running || archived}
          title={archived ? "Restore the schedule to run it" : undefined}
        >
          <Rocket className="size-3.5" /> {running ? "Running…" : "Run now"}
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="flex-1 gap-1.5"
          onClick={() => void handleToggleActive()}
          disabled={toggling || archived}
          title={archived ? "Restore the schedule first" : undefined}
        >
          {schedule.active ? (
            <>
              <Pause className="size-3.5" /> Pause
            </>
          ) : (
            <>
              <Play className="size-3.5" /> Resume
            </>
          )}
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="flex-1 gap-1.5"
          onClick={() => void handleToggleArchived()}
          disabled={archiving}
        >
          {archived ? (
            <>
              <Undo2 className="size-3.5" /> Restore
            </>
          ) : (
            <>
              <Archive className="size-3.5" /> Archive
            </>
          )}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          aria-label="Delete schedule"
          className="text-destructive hover:text-destructive"
          onClick={() => onDeleteRequest(schedule)}
        >
          <Trash2 className="size-3.5" />
        </Button>
      </div>
    </DetailPanel>
  );
}
