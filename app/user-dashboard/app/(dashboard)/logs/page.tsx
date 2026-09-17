"use client";

import { useState, useMemo } from "react";
import { type ColumnDef } from "@tanstack/react-table";
import { PageHeader } from "@/components/custom/page-header";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { DataTable } from "@/components/custom/data-table";
import { DetailPanel } from "@/components/custom/detail-panel";
import { format } from "date-fns";
import {
  ScrollText,
  AlertTriangle,
  Info,
  Bug,
  XCircle,
  Loader2,
} from "lucide-react";
import { useLogs } from "@/hooks/use-logs";
import type { AuditLog } from "@/hooks/use-logs";

type Level = "debug" | "info" | "warn" | "error";

const LEVEL_ICONS: Record<Level, React.ComponentType<{ className?: string }>> = {
  debug: Bug,
  info: Info,
  warn: AlertTriangle,
  error: XCircle,
};

const LEVEL_COLORS: Record<Level, string> = {
  debug: "bg-muted text-muted-foreground",
  info: "bg-info/15 text-info",
  warn: "bg-warning/15 text-warning",
  error: "bg-destructive/15 text-destructive",
};

// Audit-log entries are recorded system actions — surfaced here as `info`
// log lines. The event type is treated as the log source for the existing
// UI (filter/search by it).
function toLog(entry: AuditLog): {
  id: string;
  level: Level;
  message: string;
  source: string;
  timestamp: string;
  metadata: Record<string, unknown>;
} {
  return {
    id: entry.id,
    level: "info",
    message: entry.message,
    source: entry.eventType,
    timestamp: entry.occurredAt,
    metadata: {
      ...(entry.actorName ? { actorName: entry.actorName } : {}),
      ...(entry.userId ? { userId: entry.userId } : {}),
      ...(entry.projectId ? { projectId: entry.projectId } : {}),
      ...(entry.metadata ? { details: entry.metadata } : {}),
    },
  };
}
type LogRow = ReturnType<typeof toLog>;

const logColumns: ColumnDef<LogRow>[] = [
  {
    accessorKey: "level",
    header: "Level",
    cell: ({ row }) => {
      const Icon = LEVEL_ICONS[row.original.level];
      return (
        <Badge variant="secondary" className={`text-[10px] font-medium ${LEVEL_COLORS[row.original.level]}`}>
          <Icon className="size-2.5 mr-1" />
          {row.original.level.toUpperCase()}
        </Badge>
      );
    },
  },
  {
    accessorKey: "message",
    header: "Message",
    cell: ({ row }) => (
      <span className="block max-w-[400px] truncate text-sm text-foreground">
        {row.original.message}
      </span>
    ),
  },
  {
    accessorKey: "source",
    header: "Source",
    cell: ({ row }) => (
      <span className="font-mono text-xs text-muted-foreground">{row.original.source}</span>
    ),
  },
  {
    accessorKey: "timestamp",
    header: () => <span className="block text-right">Timestamp</span>,
    cell: ({ row }) => (
      <span className="block whitespace-nowrap text-right text-xs tabular-nums text-muted-foreground">
        {format(new Date(row.original.timestamp), "MMM d, HH:mm:ss")}
      </span>
    ),
  },
];

export default function LogsPage() {
  const { logs, loading, error, refresh } = useLogs();
  const [selectedLog, setSelectedLog] = useState<LogRow | null>(null);

  const renderedLogs = useMemo(() => logs.map(toLog), [logs]);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Logs"
        description={`${logs.length} recorded actions`}
        actions={
          <Button size="sm" variant="outline" className="gap-1.5" onClick={refresh}>
            <ScrollText className="size-3.5" /> Refresh
          </Button>
        }
      />

      {/* Logs table */}
      <div>
        {loading ? (
          <div className="overflow-hidden rounded-lg border bg-card">
            <div className="flex items-center justify-center py-16 text-muted-foreground">
              <Loader2 className="size-5 animate-spin mr-2" /> Loading logs…
            </div>
          </div>
        ) : error ? (
          <div className="overflow-hidden rounded-lg border bg-card">
            <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
              <AlertTriangle className="size-8 text-warning" />
              <p className="text-sm text-muted-foreground">{error}</p>
              <Button size="sm" variant="outline" onClick={refresh}>Retry</Button>
            </div>
          </div>
        ) : (
          <DataTable
            columns={logColumns}
            data={renderedLogs}
            searchPlaceholder="Search actions..."
            emptyMessage="No log entries match."
            tableClassName="bg-card"
            onRowClick={setSelectedLog}
          />
        )}

      {/* Log detail slide-over */}
      {selectedLog && (
        <DetailPanel
          key={selectedLog.id}
          onClose={() => setSelectedLog(null)}
          header={
            <div className="flex items-center gap-2">
              <span className="font-semibold text-foreground">Log Entry</span>
              <Badge variant="secondary" className={`text-xs ${LEVEL_COLORS[selectedLog.level]}`}>
                {selectedLog.level.toUpperCase()}
              </Badge>
            </div>
          }
        >
          <div className="min-h-0 flex-1 overflow-y-auto p-5">
            <div className="space-y-3 text-sm">
              <div>
                <span className="text-muted-foreground">Message</span>
                <p className="mt-1">{selectedLog.message}</p>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <span className="text-muted-foreground">Source</span>
                  <p className="font-mono text-xs mt-1">{selectedLog.source}</p>
                </div>
                <div>
                  <span className="text-muted-foreground">Timestamp</span>
                  <p className="text-xs mt-1">{format(new Date(selectedLog.timestamp), "MMM d, yyyy HH:mm:ss.SSS")}</p>
                </div>
              </div>
              <div>
                <span className="text-muted-foreground">Metadata</span>
                <pre className="mt-1 p-2 bg-muted rounded text-xs overflow-x-auto">
                  {JSON.stringify(selectedLog.metadata, null, 2)}
                </pre>
              </div>
            </div>
          </div>
        </DetailPanel>
      )}
      </div>
    </div>
  );
}
