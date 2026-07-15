"use client";

import { useState, useCallback } from "react";
import { useLogs } from "@/hooks/use-logs";
import { PageHeader } from "@/components/dashboard/shared/page-header";
import { LogsTable } from "@/components/dashboard/logs/logs-table";
import { LogInspectDrawer } from "@/components/dashboard/logs/log-inspect-drawer";
import { LogsEmptyState } from "@/components/dashboard/logs/logs-empty-state";
import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight } from "lucide-react";
import type { LogEntry } from "@/lib/types";

export default function LogsPage() {
  const {
    logs,
    loading,
    page,
    totalPages,
    setPage,
    selectedLog,
    setSelectedLog,
  } = useLogs();

  const handleSelect = useCallback(
    (log: LogEntry) => setSelectedLog(log),
    [setSelectedLog],
  );

  const handleCloseDrawer = useCallback(
    () => setSelectedLog(null),
    [setSelectedLog],
  );

  if (!loading && logs.length === 0) {
    return (
      <div className="mx-auto w-full max-w-7xl p-5">
        <PageHeader title="Logs & Activity" description="Real-time stream of incoming notification requests." />
        <LogsEmptyState />
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-7xl space-y-4 p-5">
      <PageHeader title="Logs & Activity" description="Real-time stream of incoming notification requests." />

      <LogsTable logs={logs} loading={loading} onSelect={handleSelect} />

      {/* Pagination */}
      <div className="flex items-center justify-between rounded-xl border border-border/40 bg-card px-4 py-3">
        <p className="text-sm text-muted-foreground">
          Page {page + 1} of {totalPages}
        </p>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={page === 0}
            onClick={() => setPage(page - 1)}
          >
            <ChevronLeft className="h-4 w-4" />
            Previous
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={page >= totalPages - 1}
            onClick={() => setPage(page + 1)}
          >
            Next
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Inspect Drawer */}
      <LogInspectDrawer log={selectedLog} onClose={handleCloseDrawer} />
    </div>
  );
}
