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
} from "lucide-react";

type LogEntry = {
  id: string;
  level: "debug" | "info" | "warn" | "error";
  message: string;
  source: string;
  timestamp: string;
  metadata?: Record<string, unknown>;
};

function generateLogs(count: number): LogEntry[] {
  const sources = ["notification_worker", "api_gateway", "email_service", "sms_service", "push_service", "webhook_service"];
  const levels: LogEntry["level"][] = ["debug", "info", "info", "info", "warn", "error"];
  const messages = {
    debug: ["Processing batch of 50 notifications", "Cache hit for template tpl_1", "Worker idle, waiting for tasks"],
    info: ["Notification ntf_0001 sent via email", "Provider health check passed", "Rate limit reset for provider prv_1", "Campaign cmp_001 started"],
    warn: ["Rate limit approaching for provider prv_2", "Retry attempt 2/3 for notification ntf_0042", "High latency detected: 850ms", "Quota 80% used for SMS provider"],
    error: ["Provider prv_3 returned 503", "Failed to connect to email service", "Notification ntf_0099 delivery failed", "Authentication error with provider prv_1"],
  };

  return Array.from({ length: count }, (_, i) => {
    const level = levels[i % levels.length];
    const levelMessages = messages[level];
    const timestamp = new Date();
    timestamp.setMinutes(timestamp.getMinutes() - i * 2);

    return {
      id: `log_${String(i + 1).padStart(5, "0")}`,
      level,
      message: levelMessages[i % levelMessages.length],
      source: sources[i % sources.length],
      timestamp: timestamp.toISOString(),
      metadata: i % 5 === 0 ? { notificationId: `ntf_${String(Math.floor(Math.random() * 100) + 1).padStart(4, "0")}` } : undefined,
    };
  });
}

const LOGS = generateLogs(100);

const LEVEL_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  debug: Bug,
  info: Info,
  warn: AlertTriangle,
  error: XCircle,
};

const LEVEL_COLORS: Record<string, string> = {
  debug: "bg-muted text-muted-foreground",
  info: "bg-info/15 text-info",
  warn: "bg-warning/15 text-warning",
  error: "bg-destructive/15 text-destructive",
};

export default function LogsPage() {
  const [search, setSearch] = useState("");
  const [selectedLevel, setSelectedLevel] = useState<string | null>(null);
  const [selectedLog, setSelectedLog] = useState<LogEntry | null>(null);

  const filteredLogs = useMemo(() => {
    let filtered = [...LOGS];
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
  }, [search, selectedLevel]);

  const levelCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    LOGS.forEach((l) => {
      counts[l.level] = (counts[l.level] || 0) + 1;
    });
    return counts;
  }, []);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Logs"
        description={`${LOGS.length} log entries`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Logs" }]}
        actions={
          <Button size="sm" variant="outline" className="gap-1.5">
            <ScrollText className="size-3.5" /> Export
          </Button>
        }
      />

      {/* Level filter buttons */}
      <div className="flex items-center gap-2 flex-wrap">
        {(["error", "warn", "info", "debug"] as const).map((level) => (
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
          placeholder="Search logs..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="pl-8 h-8 text-sm"
        />
      </div>

      {/* Logs table */}
      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[60px]">Level</TableHead>
                <TableHead>Message</TableHead>
                <TableHead>Source</TableHead>
                <TableHead>Timestamp</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredLogs.slice(0, 50).map((log) => {
                const Icon = LEVEL_ICONS[log.level];
                return (
                  <TableRow
                    key={log.id}
                    className="cursor-pointer hover:bg-muted/30"
                    onClick={() => setSelectedLog(log)}
                  >
                    <TableCell>
                      <Badge variant="secondary" className={`text-[10px] ${LEVEL_COLORS[log.level]}`}>
                        <Icon className="size-2.5 mr-1" />
                        {log.level.toUpperCase()}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-sm max-w-[400px] truncate">{log.message}</TableCell>
                    <TableCell>
                      <span className="text-xs text-muted-foreground font-mono">{log.source}</span>
                    </TableCell>
                    <TableCell>
                      <span className="text-xs text-muted-foreground whitespace-nowrap">
                        {format(new Date(log.timestamp), "MMM d, HH:mm:ss")}
                      </span>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
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
              {selectedLog.metadata && (
                <div>
                  <span className="text-muted-foreground">Metadata</span>
                  <pre className="mt-1 p-2 bg-muted rounded text-xs overflow-x-auto">
                    {JSON.stringify(selectedLog.metadata, null, 2)}
                  </pre>
                </div>
              )}
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
