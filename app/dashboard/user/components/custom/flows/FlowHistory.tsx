"use client";

/** The builder's History tab: the flow's run list (n8n's Executions
 * analogue) plus a per-run detail view replaying the server's steps
 * trace — one entry per node outcome, so "what breaks and where" is
 * readable after the fact. Read-only; the board stays mounted under
 * this view (unsaved edits survive the round-trip). */

import { useCallback, useEffect, useState } from "react";
import {
  ArrowLeft,
  CheckCircle2,
  FileText,
  FlaskConical,
  Globe,
  RefreshCw,
  Send,
  XCircle,
  Zap,
} from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

type RunSummary = {
  id: string;
  mode: string;
  status: string;
  error?: string | null;
  startedAt: string;
  finishedAt?: string | null;
  sendId?: string | null;
};

type RunStep = {
  nodeId: string;
  kind: string;
  label: string;
  status: string;
  detail?: string | null;
  ms: number;
};

type RunDetail = RunSummary & { steps: RunStep[] };

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

function formatDuration(startedAt: string, finishedAt?: string | null): string {
  if (!finishedAt) return "—";
  const ms = new Date(finishedAt).getTime() - new Date(startedAt).getTime();
  if (ms < 1000) return `${Math.max(0, Math.round(ms))}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

function ModeTag({ mode }: { mode: string }) {
  const label =
    mode === "webhook" ? "Webhook" : mode === "schedule" ? "Scheduled" : "Event";
  return (
    <Badge variant="secondary" className="shrink-0 text-[10px] font-medium">
      {label}
    </Badge>
  );
}

function StepIcon({ kind }: { kind: string }) {
  const cls = "mt-0.5 size-3.5 shrink-0 text-muted-foreground dark:text-zinc-500";
  if (kind === "trigger") return <Zap className={cls} />;
  if (kind === "template") return <FileText className={cls} />;
  if (kind === "provider") return <Send className={cls} />;
  if (kind === "http-request") return <Globe className={cls} />;
  return <FlaskConical className={cls} />;
}

/** The steps trace: one row per node outcome. */
function StepsTrace({ steps }: { steps: RunStep[] }) {
  return (
    <div className="divide-y rounded-lg border border-border dark:border-white/10">
      {steps.map((step, i) => (
        <div key={`${step.nodeId}-${i}`} className="flex items-start gap-2.5 px-3 py-2">
          <StepIcon kind={step.kind} />
          <div className="min-w-0 flex-1">
            <div className="flex items-center justify-between gap-2">
              <span className="min-w-0 truncate text-xs font-medium text-foreground dark:text-zinc-100">
                {step.label || step.kind}
              </span>
              <span className="shrink-0 text-[11px] text-muted-foreground dark:text-zinc-500">
                {step.ms > 0 ? `${step.ms}ms` : ""}
              </span>
            </div>
            {step.detail && (
              <p className="mt-0.5 text-[11px] leading-snug text-muted-foreground dark:text-zinc-400">
                {step.detail}
              </p>
            )}
          </div>
          {step.status === "ok" ? (
            <CheckCircle2 className="mt-0.5 size-3.5 shrink-0 text-emerald-500" />
          ) : (
            <XCircle className="mt-0.5 size-3.5 shrink-0 text-destructive" />
          )}
        </div>
      ))}
      {steps.length === 0 && (
        <p className="px-3 py-3 text-xs text-muted-foreground">No steps recorded.</p>
      )}
    </div>
  );
}

/** One run's detail: outcome banner + the steps trace. */
function RunDetailBody({
  projectId,
  flowId,
  runId,
  onBack,
}: {
  projectId: string;
  flowId: string;
  runId: string;
  onBack: () => void;
}) {
  const [run, setRun] = useState<RunDetail | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let alive = true;
    api<{ run: RunDetail | null }>(
      `/app/flows/${flowId}/runs/${runId}?project_id=${projectId}`,
    )
      .then((res) => {
        if (alive) setRun(res.run);
      })
      .catch(() => {
        if (alive) setFailed(true);
      });
    return () => {
      alive = false;
    };
  }, [projectId, flowId, runId]);

  if (failed) {
    return <p className="px-1 py-6 text-sm text-muted-foreground">Couldn&apos;t load the run.</p>;
  }
  if (!run) {
    return <div className="h-40 animate-pulse rounded-lg bg-muted dark:bg-white/5" />;
  }
  const failed_ = run.status === "failed";
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="sm" className="gap-1.5" onClick={onBack}>
          <ArrowLeft className="size-3.5" /> All runs
        </Button>
      </div>
      <div className="rounded-lg border border-border bg-popover p-3 dark:border-white/10 dark:bg-white/5">
        <div className="flex items-center justify-between gap-2">
          <span
            className={cn(
              "flex items-center gap-1.5 text-sm font-semibold",
              failed_ ? "text-destructive" : "text-foreground dark:text-zinc-100",
            )}
          >
            {failed_ ? (
              <XCircle className="size-4" />
            ) : (
              <CheckCircle2 className="size-4 text-emerald-500" />
            )}
            {failed_ ? "Failed" : "Succeeded"}
          </span>
          <ModeTag mode={run.mode} />
        </div>
        <p className="mt-1 text-[11px] text-muted-foreground dark:text-zinc-500">
          {timeAgo(run.startedAt)} · {formatDuration(run.startedAt, run.finishedAt)}
        </p>
        {run.error && (
          <p className="mt-2 rounded-md bg-destructive/10 px-2.5 py-1.5 text-xs leading-snug text-destructive">
            {run.error}
          </p>
        )}
      </div>
      <p className="text-xs font-medium text-muted-foreground dark:text-zinc-400">Steps</p>
      <StepsTrace steps={run.steps ?? []} />
    </div>
  );
}

export function FlowHistory({ projectId, flowId }: { projectId: string; flowId: string }) {
  const [runs, setRuns] = useState<RunSummary[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [openRun, setOpenRun] = useState<string | null>(null);

  const load = useCallback(() => {
    api<{ runs: RunSummary[] }>(`/app/flows/${flowId}/runs?project_id=${projectId}&limit=50`)
      .then((res) => {
        setRuns(res.runs);
        setFailed(false);
      })
      .catch(() => setFailed(true));
  }, [projectId, flowId]);

  // Reload whenever the history opens (fresh data on every tab switch).
  useEffect(() => {
    load();
  }, [load]);

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-y-auto bg-background dark:bg-[#0b0b0e]">
      <div className="mx-auto w-full max-w-xl px-4 py-6">
        {openRun ? (
          <RunDetailBody
            key={openRun}
            projectId={projectId}
            flowId={flowId}
            runId={openRun}
            onBack={() => setOpenRun(null)}
          />
        ) : (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-semibold text-foreground dark:text-zinc-100">
                Runs
              </h2>
              <Button variant="ghost" size="sm" className="gap-1.5" onClick={load}>
                <RefreshCw className="size-3.5" /> Refresh
              </Button>
            </div>
            {failed ? (
              <p className="px-1 py-6 text-sm text-muted-foreground">
                Couldn&apos;t load the run history — try refreshing.
              </p>
            ) : runs === null ? (
              <div className="h-24 animate-pulse rounded-lg bg-muted dark:bg-white/5" />
            ) : runs.length === 0 ? (
              <div className="rounded-lg border border-dashed border-border py-10 text-center dark:border-white/15">
                <FlaskConical className="mx-auto size-6 text-muted-foreground dark:text-zinc-500" />
                <p className="mt-2 text-sm text-muted-foreground dark:text-zinc-400">
                  No runs yet — this flow hasn&apos;t fired.
                </p>
                <p className="mt-1 text-[11px] text-muted-foreground dark:text-zinc-500">
                  Webhook fires, schedule runs, and event sends all land here.
                </p>
              </div>
            ) : (
              <div className="divide-y overflow-hidden rounded-lg border border-border dark:border-white/10">
                {runs.map((run) => (
                  <button
                    key={run.id}
                    type="button"
                    onClick={() => setOpenRun(run.id)}
                    className="flex w-full cursor-pointer items-center gap-3 bg-popover px-3 py-2.5 text-left transition-colors hover:bg-muted dark:bg-white/5 dark:hover:bg-white/10"
                  >
                    {run.status === "failed" ? (
                      <XCircle className="size-4 shrink-0 text-destructive" />
                    ) : (
                      <CheckCircle2 className="size-4 shrink-0 text-emerald-500" />
                    )}
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-xs font-medium text-foreground dark:text-zinc-100">
                        {run.error ?? "Delivered"}
                      </span>
                      <span className="block text-[11px] text-muted-foreground dark:text-zinc-500">
                        {timeAgo(run.startedAt)} ·{" "}
                        {formatDuration(run.startedAt, run.finishedAt)}
                      </span>
                    </span>
                    <ModeTag mode={run.mode} />
                  </button>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
