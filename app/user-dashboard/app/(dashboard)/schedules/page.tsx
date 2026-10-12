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
import { format, formatDistanceToNow } from "date-fns";
import {
  CalendarClock,
  CircleCheck,
  Clock,
  Pause,
  Play,
  Plus,
  Rocket,
  AlertTriangle,
  Loader2,
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
import { useFlows } from "@/hooks/use-flows";
import { usePendingFlow } from "@/hooks/use-pending-flow";
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
  const { flows, refresh: refreshFlows } = useFlows();

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<ServerSchedule | null>(null);
  const [form, setForm] = useState<ScheduleFormValue>(blankScheduleForm);
  const [submitting, setSubmitting] = useState(false);
  const pendingFlow = usePendingFlow(
    projectId,
    form.name.trim() ? `${form.name.trim()} flow` : `Scheduled flow ${schedules.length + 1}`,
  );

  const selected = schedules.find((s) => s.id === selectedId) ?? null;

  const stats = useMemo(() => {
    const active = schedules.filter((s) => s.active);
    const nextTimes = active
      .map((s) => (s.nextRunAt ? new Date(s.nextRunAt).getTime() : NaN))
      .filter((t) => Number.isFinite(t));
    return {
      active: active.length,
      nextRun: nextTimes.length > 0 ? Math.min(...nextTimes) : null,
      totalRuns: schedules.reduce((acc, s) => acc + s.runCount, 0),
    };
  }, [schedules]);

  const flowOptions = useMemo(
    () => flows.map((f) => ({ id: f.id, name: f.name, definition: f.definition })),
    [flows],
  );

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
      cell: ({ row }) => (
        <Badge
          variant="secondary"
          className={
            row.original.active
              ? "border-success/20 bg-success/15 text-success"
              : ""
          }
        >
          {row.original.active ? "Active" : "Paused"}
        </Badge>
      ),
    },
  ];

  const openCreate = () => {
    setForm(blankScheduleForm());
    setCreateOpen(true);
  };

  const handleCreateFlow = async () => {
    const created = await pendingFlow.create();
    if (!created) return;
    await refreshFlows();
    setForm((f) => ({ ...f, flowId: created.id }));
    toast.success("Flow created and assigned.");
  };

  const handleDeleteLockedFlow = async () => {
    const deleted = await pendingFlow.removeLocked();
    if (deleted) setForm((f) => ({ ...f, flowId: "" }));
  };

  const closeCreate = () => {
    // Discard an unsaved just-created flow so no orphan remains.
    void pendingFlow.discard();
    setCreateOpen(false);
  };

  const handleCreate = async () => {
    if (!projectId) return;
    setSubmitting(true);
    try {
      await createSchedule(projectId, {
        name: form.name,
        flowId: form.flowId,
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
      pendingFlow.commit();
      await refresh();
      toast.success("Schedule created.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't create schedule.");
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async () => {
    if (!projectId || !deleteTarget) return;
    try {
      await deleteSchedule(projectId, deleteTarget.id);
      if (selectedId === deleteTarget.id) setSelectedId(null);
      setDeleteTarget(null);
      await refresh();
      toast.success("Schedule deleted. Its flow is back on the Flows page.");
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
                format(new Date(stats.nextRun), "MMM d, HH:mm")
              ) : (
                "—"
              )
            }
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
              data={schedules}
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
            flowOptions={flowOptions}
            onClose={() => setSelectedId(null)}
            onChanged={(next) => void handleChanged(next)}
            onDeleteRequest={setDeleteTarget}
            onFlowsChanged={() => void refreshFlows()}
          />
        )}
      </div>

      {createOpen && (
        <DetailPanel
          onClose={closeCreate}
          header={
            <div>
              <h3 className="truncate font-semibold text-foreground">New schedule</h3>
              <p className="mt-0.5 truncate text-xs text-muted-foreground">
                Run a flow on a timer. Assigned flows leave the Flows page and
                are managed here.
              </p>
            </div>
          }
        >
          <div className="min-h-0 flex-1 overflow-y-auto">
            <div className="px-5 py-4">
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
              Permanently delete {deleteTarget?.name ?? "this schedule"}? Its
              flow returns to the Flows page. This cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
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
