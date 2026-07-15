"use client";

import { useRef, useCallback } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Skeleton } from "@/components/ui/skeleton";
import { CopyButton } from "@/components/dashboard/shared/copy-button";
import { StatusBadge } from "./status-badge";
import { ChannelBadge } from "./channel-badge";
import { formatTimestamp, truncateId } from "@/lib/utils";
import type { LogEntry } from "@/lib/types";

interface LogsTableProps {
  logs: LogEntry[];
  loading: boolean;
  onSelect: (log: LogEntry) => void;
}

export function LogsTable({ logs, loading, onSelect }: LogsTableProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: loading ? 10 : logs.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 48,
    overscan: 5,
  });

  const renderRow = useCallback(
    (virtualRow: any) => {
      if (loading) {
        return (
          <TableRow key={virtualRow.key} style={{ height: virtualRow.size, transform: `translateY(${virtualRow.start}px)` }} className="absolute w-full">
            <TableCell colSpan={5} className="px-4">
              <Skeleton className="h-7 w-full" />
            </TableCell>
          </TableRow>
        );
      }

      const log = logs[virtualRow.index];
      if (!log) return null;

      return (
        <TableRow
          key={log.id}
          style={{ height: virtualRow.size, transform: `translateY(${virtualRow.start}px)` }}
          className="absolute w-full cursor-pointer hover:bg-secondary/50 transition-colors"
          onClick={() => onSelect(log)}
        >
          <TableCell className="px-4">
            <div className="flex items-center gap-1.5">
              <span className="font-mono text-xs">{truncateId(log.trackingId)}</span>
              <CopyButton value={log.trackingId} />
            </div>
          </TableCell>
          <TableCell className="px-4">
            <span className="font-mono text-xs text-muted-foreground">
              {truncateId(log.subscriberId)}
            </span>
          </TableCell>
          <TableCell className="px-4">
            <ChannelBadge channel={log.channel} />
          </TableCell>
          <TableCell className="px-4">
            <StatusBadge status={log.status} />
          </TableCell>
          <TableCell className="px-4 text-right">
            <span className="text-xs text-muted-foreground whitespace-nowrap">
              {formatTimestamp(log.timestamp)}
            </span>
          </TableCell>
        </TableRow>
      );
    },
    [logs, loading, onSelect],
  );

  return (
    <div ref={parentRef} className="overflow-auto rounded-lg border border-border" style={{ height: "600px" }}>
      <Table>
        <TableHeader className="sticky top-0 z-10 bg-card">
          <TableRow>
            <TableHead className="w-[160px] px-4">Tracking ID</TableHead>
            <TableHead className="w-[130px] px-4">Subscriber ID</TableHead>
            <TableHead className="w-[100px] px-4">Channel</TableHead>
            <TableHead className="w-[100px] px-4">Status</TableHead>
            <TableHead className="w-[180px] px-4 text-right">Timestamp</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody className="relative" style={{ height: `${virtualizer.getTotalSize()}px` }}>
          {virtualizer.getVirtualItems().map(renderRow)}
        </TableBody>
      </Table>
    </div>
  );
}
