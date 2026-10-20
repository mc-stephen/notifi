"use client";

import { useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import { type ColumnDef } from "@tanstack/react-table";
import { PageHeader } from "@/components/custom/page-header";
import { StatCard } from "@/components/custom/stat-card";
import { DataTable } from "@/components/custom/data-table";
import { DetailPanel } from "@/components/custom/detail-panel";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { format, formatDistanceToNow, isSameDay } from "date-fns";
import {
  Archive,
  CalendarClock,
  CircleCheck,
  Clock,
  Download,
  Pause,
  Play,
  Plus,
  Rocket,
  AlertTriangle,
  Loader2,
  Trash2,
  Undo2,
} from "lucide-react";
import { toast } from "sonner";
import { useProjectStore } from "@/store/project-store";
import {
  createSchedule,
  deleteSchedule,
  patchSchedule,
  useSchedules,
  type ServerSchedule,
} from "@/hooks/use-schedules";
import { getFlow } from "@/hooks/use-flows";
import { downloadJson, fileSlug } from "@/lib/utils";
import {
  UNIT_SECS,
  blankScheduleForm,
  fromLocalInput,
  localTimezone,
  ScheduleFormFields,
  type ScheduleFormValue,
} from "@/components/custom/schedules/schedule-form";
import { ScheduleDetailPanel } from "@/components/custom/schedules/schedule-detail-panel";

function fmtTime(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "—" : format(d, "MMM d, HH:mm");
}

export default function SchedulesPage() {
  const router = useRouter();
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const { schedules, loading, error, refresh } = useSchedules();

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<ServerSchedule | null>(null);
  const [form, setForm] = useState<ScheduleFormValue>(blankScheduleForm);
  const [submitting, setSubmitting] = useState(false);

  const selected = schedules.find((s) => s.id === selectedId) ?? null;

  const active = useMemo(
    () => schedules.filter((s) => s.archivedAt == null),
    [schedules],
  );
  const archived = useMemo(
    () => schedules.filter((s) => s.archivedAt != null),
    [schedules],
  );

  const stats = useMemo(() => {
    const running = active.filter((s) => s.active);
    const nextTimes = running
      .map((s) => (s.nextRunAt ? new Date(s.nextRunAt).getTime() : NaN))
      .filter((t) => Number.isFinite(t));
    const nextRun = nextTimes.length > 0 ? Math.min(...nextTimes) : null;
    // The Next run card shows "MMM d" only; when several schedules land on
    // the same day it appends the time of the run that lands last that day.
    const sameDate =
      nextRun != null
        ? nextTimes.filter((t) => isSameDay(new Date(t), new Date(nextRun)))
        : [];
    return {
      active: running.length,
      nextRun,
      nextRunDateCount: sameDate.length,
      lastRunOnDate: sameDate.length >= 2 ? Math.max(...sameDate) : null,
      totalRuns: schedules.reduce((acc, s) => acc + s.runCount, 0),
    };
  }, [active, schedules]);

  const columns: ColumnDef<ServerSchedule>[] = [
    {
      accessorKey: "name",
      header: "Schedule",
      cell: ({ row }) => (
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">{row.original.name}</div>
          <div className="truncate text-xs text-muted-foreground">
            {row.original.timezone}
          </div>
        </div>
      ),
    },
    {
      accessorKey: "flowName",
      header: "Flow",
      cell: ({ row }) => (
        <button
          type="button"
          className="truncate text-sm hover:underline"
          onClick={(e) => {
            e.stopPropagation();
            router.push(`/flows/${row.original.flowId}`);
          }}
        >
          {row.original.flowName || "—"}
        </button>
      ),
    },
    {
      accessorKey: "timing",
      header: "Timing",
      cell: ({ row }) => <span className="text-xs">{row.original.timing}</span>,
    },
    {
      accessorKey: "nextRunAt",
      header: "Next Run",
      cell: ({ row }) => (
        <span className="text-xs whitespace-nowrap text-muted-foreground">
          {fmtTime(row.original.nextRunAt)}
        </span>
      ),
    },
    {
      accessorKey: "lastRunAt",
      header: "Last Run",
      cell: ({ row }) => {
        const s = row.original;
        return (
          <span className="text-xs whitespace-nowrap text-muted-foreground">
            {fmtTime(s.lastRunAt)}
            {s.lastStatus === "failed" ? (
              <span className="text-destructive"> · failed</span>
            ) : s.lastStatus === "completed" ? (
              <span className="text-success"> · ok</span>
            ) : null}
          </span>
        );
      },
    },
    {
      accessorKey: "active",
      header: "Status",
      cell: ({ row }) => {
        const s = row.original;
        if (s.archivedAt != null) {
          return <Badge className="border-warning/20 bg-warning/15 text-warning">Archived</Badge>;
        }
        return (
          <Badge
            variant="secondary"
            className={
              s.active ? "border-success/20 bg-success/15 text-success" : ""
            }
          >
            {s.active ? "Active" : "Paused"}
          </Badge>
        );
      },
    },
  ];

  const openCreate = () => {
    setForm(blankScheduleForm());
    setCreateOpen(true);
  };

  const closeCreate = () => setCreateOpen(false);

  const handleCreate = async () => {
    if (!projectId) return;
    setSubmitting(true);
    try {
      await createSchedule(projectId, {
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
        templateId: form.templateId || null,
        recipients: [],
      });
      setCreateOpen(false);
      setForm(blankScheduleForm());
      await refresh();
      toast.success("Schedule created — its flow is ready in the builder.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't create schedule.");
    } finally {
      setSubmitting(false);
    }
  };

  const toggleArchived = async (schedule: ServerSchedule, restore: boolean) => {
    if (!projectId) return;
    try {
      await patchSchedule(projectId, schedule.id, { archived: restore });
      await refresh();
      toast.success(restore ? "Schedule restored." : "Schedule archived.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't update schedule.");
    }
  };

  /** Exports the schedule's flow before a terminal delete. */
  const downloadFlowData = async (schedule: ServerSchedule) => {
    if (!projectId) return;
    try {
      const flow = await getFlow(projectId, schedule.flowId);
      downloadJson(`notifi-flow-${fileSlug(schedule.flowName || schedule.name)}.json`, {
        kind: "notifi-flow-export",
        exportedAt: new Date().toISOString(),
        schedule: {
          name: schedule.name,
          timing: schedule.timing,
          templateId: schedule.templateId,
          channel: schedule.channel,
          title: schedule.title,
          body: schedule.body,
          recipients: schedule.recipients,
        },
        flow: {
          name: flow.name,
          definition: flow.definition,
          version: flow.version,
        },
      });
      toast.success("Flow data downloaded.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't export flow data.");
    }
  };

  const handleDelete = async () => {
    if (!projectId || !deleteTarget) return;
    try {
      await deleteSchedule(projectId, deleteTarget.id);
      if (selectedId === deleteTarget.id) setSelectedId(null);
      setDeleteTarget(null);
      await refresh();
      toast.success("Schedule deleted. Its flow went with it.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't delete schedule.");
    }
  };

  const handleChanged = async (next: ServerSchedule) => {
    setSelectedId(next.id);
    await refresh();
  };

  return (
    <div className="space-y-6">
      <PageHeader
        title="Schedules"
        description={`${stats.active} active schedule${stats.active === 1 ? "" : "s"}`}
        actions={
          <Button size="sm" className="gap-1.5" onClick={openCreate} disabled={!projectId}>
            <Plus className="size-3.5" /> New schedule
          </Button>
        }
      />

      {!error && (schedules.length > 0 || loading) && (
        <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
          <StatCard
            label="Active schedules"
            value={loading ? "–" : stats.active}
            icon={Clock}
          />
          <StatCard
            label="Next run"
            value={
              loading ? (
                "–"
              ) : stats.nextRun ? (
                stats.nextRunDateCount >= 2 && stats.lastRunOnDate != null ? (
                  format(new Date(stats.lastRunOnDate), "MMM d, HH:mm")
                ) : (
                  format(new Date(stats.nextRun), "MMM d")
                )
              ) : (
                "—"
              )
            }
            valueClassName="text-2xl"
            sub={
              !loading && stats.nextRun
                ? formatDistanceToNow(new Date(stats.nextRun), { addSuffix: true })
                : undefined
            }
            icon={CalendarClock}
          />
          <StatCard
            label="Total runs"
            value={loading ? "–" : stats.totalRuns}
            icon={Rocket}
          />
          <StatCard
            label="Succeeded"
            value={
              loading
                ? "–"
                : schedules.filter((s) => s.lastStatus === "completed").length
            }
            sub="Schedules whose last run completed"
            icon={CircleCheck}
          />
        </div>
      )}

      <div className="flex items-start">
        <div className="min-w-0 flex-1">
          {loading ? (
            <div className="overflow-hidden rounded-lg border bg-card">
              <div className="flex items-center justify-center py-16 text-muted-foreground">
                <Loader2 className="mr-2 size-5 animate-spin" /> Loading schedules…
              </div>
            </div>
          ) : error ? (
            <div className="overflow-hidden rounded-lg border bg-card">
              <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
                <AlertTriangle className="size-8 text-warning" />
                <p className="text-sm text-muted-foreground">{error}</p>
                <Button size="sm" variant="outline" onClick={() => void refresh()}>
                  Retry
                </Button>
              </div>
            </div>
          ) : (
            <DataTable
              columns={columns}
              data={active}
              searchPlaceholder="Search schedules..."
              emptyMessage="No schedules yet. Create one to run a flow on a timer."
              onRowClick={(row) => setSelectedId(row.id)}
              rowActions={(schedule) => (
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    title={schedule.active ? "Pause" : "Resume"}
                    onClick={(e) => {
                      e.stopPropagation();
                      if (!projectId) return;
                      void (async () => {
                        try {
                          await patchSchedule(projectId, schedule.id, {
                            active: !schedule.active,
                          });
                          await refresh();
                          toast.success(
                            schedule.active ? "Schedule paused." : "Schedule resumed.",
                          );
                        } catch (err) {
                          toast.error(
                            err instanceof Error ? err.message : "Couldn't update schedule.",
                          );
                        }
                      })();
                    }}
                  >
                    {schedule.active ? (
                      <Pause className="size-3" />
                    ) : (
                      <Play className="size-3" />
                    )}
                  </Button>
                </div>
              )}
            />
          )}
        </div>

        {selected && projectId && (
          <ScheduleDetailPanel
            key={selected.id}
            schedule={selected}
            projectId={projectId}
            onClose={() => setSelectedId(null)}
            onChanged={(next) => void handleChanged(next)}
            onDeleteRequest={setDeleteTarget}
          />
        )}
      </div>

      {!error && !loading && archived.length > 0 && (
        <section className="space-y-2">
          <div className="flex items-center gap-2 px-0.5">
            <Archive className="size-3.5 text-muted-foreground" />
            <h2 className="text-sm font-medium text-muted-foreground">Archived</h2>
            <span className="text-xs text-muted-foreground">
              {archived.length} parked · not counted in stats
            </span>
          </div>
          <DataTable
            columns={columns}
            data={archived}
            searchPlaceholder="Search archived..."
            emptyMessage="Nothing archived."
            onRowClick={(row) => setSelectedId(row.id)}
            rowActions={(schedule) => (
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="icon-xs"
                  title="Restore"
                  onClick={(e) => {
                    e.stopPropagation();
                    void toggleArchived(schedule, true);
                  }}
                >
                  <Undo2 className="size-3" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-xs"
                  title="Delete"
                  onClick={(e) => {
                    e.stopPropagation();
                    setDeleteTarget(schedule);
                  }}
                >
                  <Trash2 className="size-3" />
                </Button>
              </div>
            )}
          />
        </section>
      )}

      {createOpen && (
        <DetailPanel
          onClose={closeCreate}
          header={
            <div>
              <h3 className="truncate font-semibold text-foreground">New schedule</h3>
              <p className="mt-0.5 truncate text-xs text-muted-foreground">
                Run a flow on a timer. The schedule creates and owns its flow —
                edit it in the builder afterwards.
              </p>
            </div>
          }
        >
          <div className="min-h-0 flex-1 overflow-y-auto">
            <div className="px-5 py-4">
              <ScheduleFormFields value={form} onChange={setForm} />
            </div>
          </div>
          <div className="flex gap-2 border-t border-border p-4">
            <Button
              variant="outline"
              size="sm"
              className="flex-1"
              onClick={closeCreate}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              className="flex-1"
              onClick={() => void handleCreate()}
              disabled={submitting}
            >
              {submitting ? "Creating…" : "Create schedule"}
            </Button>
          </div>
        </DetailPanel>
      )}

      <Dialog open={!!deleteTarget} onOpenChange={() => setDeleteTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete schedule</DialogTitle>
            <DialogDescription>
              Permanently delete {deleteTarget?.name ?? "this schedule"} <em>and
              its flow</em>? The flow is deleted too — nothing can ever fire
              from it again, and this cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <p className="rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning">
            Deleting also erases everything inside the flow. If you may want it
            back, archive instead — archived schedules keep their flow and
            stats, they just stop firing.
          </p>
          <DialogFooter className="items-center">
            <Button
              variant="ghost"
              size="sm"
              className="gap-1.5"
              disabled={!deleteTarget}
              onClick={() => {
                if (deleteTarget) void downloadFlowData(deleteTarget);
              }}
            >
              <Download className="size-3.5" /> Download flow data
            </Button>
            <div className="flex-1" />
            {deleteTarget?.archivedAt == null && (
              <Button
                variant="outline"
                onClick={() => {
                  void toggleArchived(deleteTarget!, true);
                  setDeleteTarget(null);
                }}
              >
                <Archive className="mr-1.5 size-3.5" /> Archive instead
              </Button>
            )}
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={() => void handleDelete()}>
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
