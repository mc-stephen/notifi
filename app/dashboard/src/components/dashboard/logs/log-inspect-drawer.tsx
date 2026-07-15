"use client";

import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import { Badge } from "@/components/ui/badge";
import { JsonViewer } from "./json-viewer";
import { ExecutionTimeline } from "./execution-timeline";
import { StatusBadge } from "./status-badge";
import { ChannelBadge } from "./channel-badge";
import { CopyButton } from "@/components/dashboard/shared/copy-button";
import { Calendar, Hash, User } from "lucide-react";
import type { LogEntry } from "@/lib/types";

interface LogInspectDrawerProps {
  log: LogEntry | null;
  onClose: () => void;
}

export function LogInspectDrawer({ log, onClose }: LogInspectDrawerProps) {
  if (!log) return null;

  return (
    <Sheet open={!!log} onOpenChange={(open) => !open && onClose()}>
      <SheetContent side="right" className="w-full sm:max-w-lg overflow-y-auto">
        <SheetHeader>
          <SheetTitle className="flex items-center gap-2">
            <span className="font-mono text-sm">Log Details</span>
            <CopyButton value={log.trackingId} />
          </SheetTitle>
          <SheetDescription>
            <div className="flex items-center gap-2 mt-1">
              <StatusBadge status={log.status} />
              <ChannelBadge channel={log.channel} />
            </div>
          </SheetDescription>
        </SheetHeader>

        <div className="space-y-6 p-4 pt-2">
          {/* Error section */}
          {log.error && (
            <div className="rounded-xl border border-red-500/20 bg-red-500/5 p-4">
              <p className="text-xs font-semibold text-red-500 uppercase tracking-wider">Error</p>
              <p className="mt-1.5 text-sm text-red-400">{log.error}</p>
            </div>
          )}

          {/* Key metadata */}
          <div className="grid grid-cols-2 gap-3">
            <div className="rounded-lg border border-border/40 bg-secondary/30 p-3">
              <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground uppercase tracking-wider mb-1">
                <Hash className="h-3 w-3" />
                Tracking ID
              </div>
              <span className="font-mono text-xs text-foreground">{log.trackingId}</span>
            </div>
            <div className="rounded-lg border border-border/40 bg-secondary/30 p-3">
              <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground uppercase tracking-wider mb-1">
                <User className="h-3 w-3" />
                Subscriber
              </div>
              <span className="font-mono text-xs text-foreground">{log.subscriberId}</span>
            </div>
            <div className="col-span-2 rounded-lg border border-border/40 bg-secondary/30 p-3">
              <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground uppercase tracking-wider mb-1">
                <Calendar className="h-3 w-3" />
                Timestamp
              </div>
              <span className="text-xs text-foreground">{new Date(log.timestamp).toLocaleString()}</span>
            </div>
          </div>

          {/* Execution Timeline */}
          <div>
            <h4 className="mb-3 text-sm font-semibold">Execution Timeline</h4>
            <ExecutionTimeline steps={log.executionSteps} />
          </div>

          {/* JSON Metadata */}
          <div>
            <h4 className="mb-3 text-sm font-semibold">Request Metadata</h4>
            <JsonViewer data={log.metadata} />
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}
