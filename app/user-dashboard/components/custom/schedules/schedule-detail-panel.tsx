"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { format, formatDistanceToNow } from "date-fns";
import { ArrowUpRight, Pause, Play, Rocket, Trash2 } from "lucide-react";
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
import { usePendingFlow } from "@/hooks/use-pending-flow";
import { toast } from "sonner";

function fmtTime(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "—" : format(d, "MMM d, HH:mm");
}

/**
 * Right-side schedule editor: run metadata, full field form, and an
 * Open-flow button deep-linking to the assigned flow's builder.
 */
export function ScheduleDetailPanel({
  schedule,
  projectId,
  flowOptions,
  onClose,
  onChanged,
  onDeleteRequest,
  onFlowsChanged,
}: {
  schedule: ServerSchedule;
  projectId: string;
  flowOptions: { id: string; name: string; definition?: unknown }[];
  onClose: () => void;
  /** Refresh the page list after a mutation (receives the updated row). */
  onChanged: (next: ServerSchedule) => void;
  onDeleteRequest: (schedule: ServerSchedule) => void;
  /** Refresh the flow options (after inline flow creation). */
  onFlowsChanged: () => void;
}) {
  const router = useRouter();
  const [form, setForm] = useState<ScheduleFormValue>(() => scheduleToForm(schedule));
  const [saving, setSaving] = useState(false);
  const [running, setRunning] = useState(false);
  const [toggling, setToggling] = useState(false);
  const pendingFlow = usePendingFlow(projectId, `${schedule.name} flow`);

  const close = () => {
    // Discard an unsaved just-created flow so no orphan remains.
    void pendingFlow.discard();
    onClose();
  };

  const handleCreateFlow = async () => {
    const created = await pendingFlow.create();
    if (!created) return;
    setForm((f) => ({ ...f, flowId: created.id }));
    onFlowsChanged();
    toast.success("Flow created and assigned.");
  };

  const handleDeleteLockedFlow = async () => {
    const deleted = await pendingFlow.removeLocked();
    if (deleted) setForm((f) => ({ ...f, flowId: "" }));
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      // Legacy (pre-template) schedules keep their stored message unless
      // a template is picked — payload fields are simply omitted then.
      const useTemplate = !!form.templateId;
      const updated = await patchSchedule(projectId, schedule.id, {
        name: form.name,
        flowId: form.flowId || undefined,
        kind: form.kind,
        runAt: form.kind === "once" ? fromLocalInput(form.runAt) : null,
        intervalSecs:
          form.kind === "recurring"
            ? form.intervalAmount * UNIT_SECS[form.intervalUnit]
            : null,
        days: form.kind === "weekly" ? form.days : [],
        runTime: form.kind === "weekly" ? form.runTime : null,
        timezone: localTimezone(),
        templateId: form.templateId || null,
        ...(useTemplate
          ? {}
          : {
              channel: schedule.channel,
              title: schedule.title,
              body: schedule.body,
              recipients: schedule.recipients,
            }),
      });
      toast.success("Schedule updated.");
      pendingFlow.commit();
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

  return (
    <DetailPanel
      onClose={close}
      header={
        <div>
          <div className="flex items-center gap-2">
            <h3 className="truncate font-semibold text-foreground">{schedule.name}</h3>
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
                {schedule.nextRunAt ? (
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
                  <span className="font-normal text-muted-foreground">
                    Stored message (pre-template)
                  </span>
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
            Open assigned flow <ArrowUpRight className="size-3.5" />
          </Button>

          <Separator />

          <ScheduleFormFields
            value={form}
            onChange={setForm}
            flowOptions={flowOptions}
            onCreateFlow={() => void handleCreateFlow()}
            creatingFlow={pendingFlow.busy}
            lockedFlowId={pendingFlow.pendingId}
            onDeleteLockedFlow={() => void handleDeleteLockedFlow()}
            deletingLocked={pendingFlow.busy}
          />

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
          disabled={running}
        >
          <Rocket className="size-3.5" /> {running ? "Running…" : "Run now"}
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="flex-1 gap-1.5"
          onClick={() => void handleToggleActive()}
          disabled={toggling}
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
