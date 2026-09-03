"use client";

import { useState, useMemo } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { format } from "date-fns";
import {
  Search,
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

const LEVELS: Level[] = ["error", "warn", "info", "debug"];

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

export default function LogsPage() {
  const { logs, loading, error, refresh } = useLogs();
  const [search, setSearch] = useState("");
  const [selectedLevel, setSelectedLevel] = useState<Level | null>(null);
  const [selectedLog, setSelectedLog] = useState<ReturnType<typeof toLog> | null>(null);

  const renderedLogs = useMemo(() => logs.map(toLog), [logs]);

  const filteredLogs = useMemo(() => {
    let filtered = [...renderedLogs];
    if (search) {
      const q = search.toLowerCase();
      filtered = filtered.filter(
        (l) =>
          l.id.toLowerCase().includes(q) ||
          l.message.toLowerCase().includes(q) ||
          l.source.toLowerCase().includes(q),
      );
    }
    if (selectedLevel) {
      filtered = filtered.filter((l) => l.level === selectedLevel);
    }
    return filtered;
  }, [renderedLogs, search, selectedLevel]);

  const levelCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    renderedLogs.forEach((l) => {
      counts[l.level] = (counts[l.level] || 0) + 1;
    });
    return counts;
  }, [renderedLogs]);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Logs"
        description={`${logs.length} recorded actions`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Logs" }]}
        actions={
          <Button size="sm" variant="outline" className="gap-1.5" onClick={refresh}>
            <ScrollText className="size-3.5" /> Refresh
          </Button>
        }
      />

      {/* Level filter buttons */}
      <div className="flex items-center gap-2 flex-wrap">
        {LEVELS.map((level) => (
          <button
            key={level}
            onClick={() => setSelectedLevel(selectedLevel === level ? null : level)}
            className={`flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm transition-colors ${
              selectedLevel === level ? "border-primary bg-primary/5" : "hover:bg-muted/50"
            }`}
          >
            <div className={`size-2 rounded-full ${LEVEL_COLORS[level].split(" ")[0]}`} />
            <span className="capitalize">{level}</span>
            <Badge variant="secondary" className="text-[10px] px-1 h-3.5">{levelCounts[level] ?? 0}</Badge>
          </button>
        ))}
        {selectedLevel && (
          <Button variant="ghost" size="sm" onClick={() => setSelectedLevel(null)} className="text-xs">
            Clear
          </Button>
        )}
      </div>

      {/* Search */}
      <div className="relative max-w-sm">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
        <Input
          placeholder="Search actions..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="pl-8 h-8 text-sm"
        />
      </div>

      {/* Logs table */}
      <Card className="pb-0">
        <CardContent className="p-0">
          {loading ? (
            <div className="flex items-center justify-center py-16 text-muted-foreground">
              <Loader2 className="size-5 animate-spin mr-2" /> Loading logs…
            </div>
          ) : error ? (
            <div className="flex flex-col items-center justify-center gap-3 py-16 text-center">
              <AlertTriangle className="size-8 text-warning" />
              <p className="text-sm text-muted-foreground">{error}</p>
              <Button size="sm" variant="outline" onClick={refresh}>Retry</Button>
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow className="hover:bg-transparent">
                  <TableHead className="w-[110px] pl-4 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    Level
                  </TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    Message
                  </TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    Source
                  </TableHead>
                  <TableHead className="pr-4 text-right text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    Timestamp
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredLogs.slice(0, 100).map((log) => {
                  const Icon = LEVEL_ICONS[log.level];
                  return (
                    <TableRow
                      key={log.id}
                      className="cursor-pointer odd:bg-white even:bg-muted/40 hover:bg-muted/70"
                      onClick={() => setSelectedLog(log)}
                    >
                      <TableCell className="pl-4">
                        <Badge variant="secondary" className={`text-[10px] font-medium ${LEVEL_COLORS[log.level]}`}>
                          <Icon className="size-2.5 mr-1" />
                          {log.level.toUpperCase()}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-sm max-w-[400px] truncate text-foreground">{log.message}</TableCell>
                      <TableCell>
                        <span className="font-mono text-xs text-muted-foreground">{log.source}</span>
                      </TableCell>
                      <TableCell className="pr-4">
                        <span className="whitespace-nowrap text-right text-xs tabular-nums text-muted-foreground">
                          {format(new Date(log.timestamp), "MMM d, HH:mm:ss")}
                        </span>
                      </TableCell>
                    </TableRow>
                  );
                })}
                {filteredLogs.length === 0 && (
                  <TableRow className="odd:bg-white even:bg-muted/40 hover:bg-transparent">
                    <TableCell colSpan={4} className="py-10 text-center text-sm text-muted-foreground">
                      No log entries match.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Log detail dialog */}
      <Dialog open={!!selectedLog} onOpenChange={() => setSelectedLog(null)}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              Log Entry
              {selectedLog && (
                <Badge variant="secondary" className={`text-xs ${LEVEL_COLORS[selectedLog.level]}`}>
                  {selectedLog.level.toUpperCase()}
                </Badge>
              )}
            </DialogTitle>
          </DialogHeader>
          {selectedLog && (
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
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
