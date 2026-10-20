"use client";

import { useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { useRouter } from "next/navigation";
import { type ColumnDef } from "@tanstack/react-table";
import { format, formatDistanceToNow } from "date-fns";
import { toast } from "sonner";
import { Plus, Trash2, Layers, Zap, CircleCheck, TriangleAlert, Loader2, Check, Copy } from "lucide-react";
import { PageHeader } from "@/components/custom/page-header";
import { StatCard } from "@/components/custom/stat-card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DataTable } from "@/components/custom/data-table";
import { EmptyState } from "@/components/custom/empty-state";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog";
import { useProjectStore } from "@/store/project-store";
import {
  adoptLegacyDrafts,
  asFlowDoc,
  createFlow,
  deleteFlow,
  handlesSendEvent,
  useFlows,
  type ServerFlow,
} from "@/hooks/use-flows";
import { validateFlow } from "@/components/custom/flows/flow-validate";

const columns: ColumnDef<ServerFlow>[] = [
  {
    accessorKey: "name",
    header: "Flow",
    cell: ({ row }) => (
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium">{row.original.name}</span>
        {handlesSendEvent(row.original.definition) && (
          <Badge variant="secondary" className="text-[10px] font-medium">
            Handles sends
          </Badge>
        )}
      </div>
    ),
  },
  {
    accessorKey: "updatedAt",
    header: "Updated",
    cell: ({ row }) => (
      <span className="whitespace-nowrap text-xs text-muted-foreground">
        {formatDistanceToNow(new Date(row.original.updatedAt), { addSuffix: true })}
      </span>
    ),
  },
  {
    accessorKey: "createdAt",
    header: "Created",
    cell: ({ row }) => (
      <span className="whitespace-nowrap text-xs text-muted-foreground">
        {format(new Date(row.original.createdAt), "MMM d, yyyy")}
      </span>
    ),
  },
  {
    id: "steps",
    accessorFn: (flow) => asFlowDoc(flow.definition)?.nodes.length ?? 0,
    header: "Steps",
    cell: ({ row }) => {
      const count = asFlowDoc(row.original.definition)?.nodes.length ?? 0;
      return (
        <span className="whitespace-nowrap text-xs text-muted-foreground tabular-nums">
          {count} step{count === 1 ? "" : "s"}
        </span>
      );
    },
  },
];

function FlowStatusBadge({ flow }: { flow: ServerFlow }) {
  const doc = asFlowDoc(flow.definition);
  if (!doc) {
    return (
      <Badge variant="secondary" className="border-destructive/20 bg-destructive/15 text-[10px] font-medium text-destructive">
        Invalid
      </Badge>
    );
  }
  const errors = validateFlow(doc.nodes, doc.edges).filter(
    (i) => i.severity === "error",
  ).length;
  return errors > 0 ? (
    <Badge
      variant="secondary"
      className="border-warning/20 bg-warning/15 text-[10px] font-medium text-warning"
    >
      {errors} issue{errors === 1 ? "" : "s"}
    </Badge>
  ) : (
    <Badge variant="outline" className="text-[10px] font-medium">
      Valid
    </Badge>
  );
}

function CopyNameChip({ name }: { name: string }) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(name);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error("Couldn't copy to clipboard.");
    }
  };

  return (
    <button
      type="button"
      onClick={() => void copy()}
      title="Copy flow name"
      aria-label="Copy flow name"
      className="inline-flex items-center gap-1 rounded border border-destructive/25 bg-destructive/10 px-1.5 py-0.5 font-mono text-xs text-destructive transition-colors hover:bg-destructive/15"
    >
      {name}
      {copied ? <Check className="size-3" /> : <Copy className="size-3 opacity-60" />}
    </button>
  );
}

export default function FlowsPage() {
  const router = useRouter();
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const { flows, loading, error, refresh } = useFlows();
  const [deleteTarget, setDeleteTarget] = useState<ServerFlow | null>(null);
  const [confirmName, setConfirmName] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [creating, setCreating] = useState(false);
  const [importing, setImporting] = useState(false);
  const importedRef = useRef(false);
  const mounted = useSyncExternalStore(
    () => () => {},
    () => true,
    () => false,
  );

  // One-time adoption of pre-server localStorage drafts.
  useEffect(() => {
    if (!projectId || !mounted || importedRef.current) return;
    importedRef.current = true;
    (async () => {
      setImporting(true);
      try {
        if (await adoptLegacyDrafts(projectId)) await refresh();
      } finally {
        setImporting(false);
      }
    })();
  }, [projectId, mounted, refresh]);

  const tableColumns: ColumnDef<ServerFlow>[] = useMemo(
    () => [
      ...columns,
      {
        id: "status",
        header: "Status",
        cell: ({ row }: { row: { original: ServerFlow } }) => (
          <FlowStatusBadge flow={row.original} />
        ),
      },
    ],
    [],
  );

  const stats = useMemo(() => {
    let handling = 0;
    let valid = 0;
    for (const flow of flows) {
      if (handlesSendEvent(flow.definition)) handling += 1;
      const doc = asFlowDoc(flow.definition);
      if (
        doc &&
        validateFlow(doc.nodes, doc.edges).every((i) => i.severity !== "error")
      ) {
        valid += 1;
      }
    }
    return { total: flows.length, handling, valid, attention: flows.length - valid };
  }, [flows]);

  const statsLoading = loading || importing;

  const statItems = [
    {
      label: "Total flows",
      value: statsLoading ? "–" : stats.total,
      icon: Layers,
    },
    {
      label: "Handling sends",
      value: statsLoading ? "–" : stats.handling,
      sub: "Eligible for send execution",
      icon: Zap,
    },
    {
      label: "Valid",
      value: statsLoading ? "–" : stats.valid,
      icon: CircleCheck,
    },
    {
      label: "Needs attention",
      value: statsLoading ? "–" : stats.attention,
      sub: "Fix before they send cleanly",
      icon: TriangleAlert,
    },
  ];

  const handleCreate = async () => {
    if (!projectId || creating) return;
    setCreating(true);
    try {
      const flow = await createFlow(projectId, `Untitled flow ${flows.length + 1}`);
      await refresh();
      router.push(`/flows/${flow.id}`);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't create flow.");
    } finally {
      setCreating(false);
    }
  };

  const openDelete = (flow: ServerFlow) => {
    setConfirmName("");
    setDeleteTarget(flow);
  };

  const closeDelete = () => {
    if (deleting) return;
    setDeleteTarget(null);
  };

  const handleDelete = async () => {
    if (!projectId || !deleteTarget || deleting) return;
    setDeleting(true);
    try {
      await deleteFlow(projectId, deleteTarget.id);
      setDeleteTarget(null);
      setConfirmName("");
      await refresh();
      toast.success("Flow deleted.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't delete flow.");
    } finally {
      setDeleting(false);
    }
  };

  return (
    <div className="space-y-6">
      <PageHeader
        title="Flows"
        description="Design how notifications route across providers — priority order, fallbacks, conditions, and AI transforms."
        actions={
          <Button size="sm" className="gap-1.5" onClick={() => void handleCreate()} disabled={!projectId || creating}>
            {creating ? <Loader2 className="size-3.5 animate-spin" /> : <Plus className="size-3.5" />}
            {creating ? "Creating…" : "New flow"}
          </Button>
        }
      />

      {!projectId ? (
        <EmptyState
          title="No project selected"
          description="Select a project to see and design its notification flows."
        />
      ) : error ? (
        <EmptyState
          title="Couldn't load flows"
          description={error}
          action={{ label: "Try again", onClick: () => void refresh() }}
        />
      ) : flows.length === 0 && !loading && !importing ? (
        <EmptyState
          title="No flows yet"
          description="Design your first notification flow — priority order, fallbacks, conditions, and AI transforms."
          action={{ label: creating ? "Creating…" : "New flow", onClick: () => void handleCreate() }}
        />
      ) : (
        <>
          {projectId && (flows.length > 0 || statsLoading) && (
            <div className="mb-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
              {statItems.map((item) => (
                <StatCard
                  key={item.label}
                  label={item.label}
                  value={item.value}
                  sub={item.sub}
                  icon={item.icon}
                />
              ))}
            </div>
          )}
          <DataTable
            columns={tableColumns}
            data={flows}
            isLoading={loading || importing}
            searchPlaceholder="Search flows..."
            emptyMessage="No flows match."
            onRowClick={(row) => router.push(`/flows/${row.id}`)}
            rowActions={(flow) => (
              <Button
                variant="ghost"
                size="icon-xs"
                title="Delete flow"
                aria-label={`Delete ${flow.name}`}
                className="text-destructive hover:text-destructive"
                onClick={(e) => {
                  e.stopPropagation();
                  openDelete(flow);
                }}
              >
                <Trash2 className="size-3.5" />
              </Button>
            )}
          />
        </>
      )}

      <Dialog open={!!deleteTarget} onOpenChange={() => closeDelete()}>
        <DialogContent className="border-destructive/25 gap-3 sm:max-w-md">
          <div className="flex items-center gap-2.5">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-destructive/30 bg-destructive/10">
              <TriangleAlert className="size-4.5 text-destructive" />
            </div>
            <DialogTitle>Delete flow</DialogTitle>
          </div>
          <DialogDescription>
            <span className="font-medium text-foreground">{deleteTarget?.name}</span>{" "}
            will be permanently deleted. Sends that relied on it will fail
            closed until another flow handles them. This cannot be undone.
          </DialogDescription>
          <div className="space-y-1.5">
            <Label htmlFor="delete-confirm-name" className="text-sm font-medium">
              Type{" "}
              {deleteTarget?.name ? (
                <CopyNameChip key={deleteTarget.id} name={deleteTarget.name} />
              ) : null}{" "}
              to confirm
            </Label>
            <Input
              id="delete-confirm-name"
              value={confirmName}
              onChange={(e) => setConfirmName(e.target.value)}
              placeholder={deleteTarget?.name ?? "Flow name"}
              className="h-9 font-mono text-sm"
              disabled={deleting}
            />
          </div>
          <DialogFooter className="gap-2">
            <Button variant="outline" onClick={closeDelete} disabled={deleting}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => void handleDelete()}
              disabled={deleting || confirmName !== (deleteTarget?.name ?? "")}
            >
              {deleting && <Loader2 className="size-3.5 animate-spin" />}
              {deleting ? "Deleting…" : "Delete flow"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
