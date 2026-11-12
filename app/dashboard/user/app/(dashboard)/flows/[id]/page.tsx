"use client";

import { use, useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { ArrowLeft, GitBranch, History } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { FlowCanvas } from "@/components/custom/flows/FlowCanvas";
import { FlowHistory } from "@/components/custom/flows/FlowHistory";
import { useProjectStore } from "@/store/project-store";
import { ApiError } from "@/lib/api";
import {
  asFlowDoc,
  getFlow,
  handlesSendEvent,
  putFlow,
  type ServerFlow,
} from "@/hooks/use-flows";
import type { FlowDoc } from "@/components/custom/flows/flow-types";

type LoadStatus = "missing" | "error";
type BuilderView = "board" | "history";

/**
 * By-id builder: schedule-owned flows never appear on the /flows list,
 * so the load path is `GET /app/flows/{id}` — the same door deep links
 * use. Per-id states derive from what's loaded; no synchronous setState
 * inside effects.
 */
export default function FlowBuilderPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  const projectId = useProjectStore((s) => s.currentProject?.id);

  /** Last successfully loaded flow + the id it belongs to. */
  const [loaded, setLoaded] = useState<{ id: string; flow: ServerFlow } | null>(
    null,
  );
  /** Terminal failure for the id it carries. */
  const [failed, setFailed] = useState<{ id: string; kind: LoadStatus } | null>(
    null,
  );

  const meta = loaded?.id === id ? loaded.flow : null;
  const failure = failed?.id === id ? failed.kind : null;
  const doc = meta ? asFlowDoc(meta.definition) : null;

  // Null name draft follows the stored name; typing takes over until
  // committed. Carries its flow id so stale typing never leaks between
  // two navigated builders.
  const [draft, setDraft] = useState<{ id: string; name: string } | null>(null);

  /** Board (builder) vs History (run list) — the top-center tab pill. */
  const [view, setView] = useState<BuilderView>("board");

  useEffect(() => {
    if (!projectId) return;
    let ignore = false;
    getFlow(projectId, id)
      .then((flow) => {
        if (ignore) return;
        setLoaded({ id, flow });
      })
      .catch((e: unknown) => {
        if (ignore) return;
        setFailed({
          id,
          kind: e instanceof ApiError && e.status === 404 ? "missing" : "error",
        });
      });
    return () => {
      ignore = true;
    };
  }, [projectId, id]);

  const commitName = useCallback(
    async (name: string, current: FlowDoc) => {
      if (!projectId) return false;
      try {
        const updated = await putFlow(projectId, id, name, current);
        setLoaded({ id, flow: updated });
        return true;
      } catch (e) {
        toast.error(e instanceof Error ? e.message : "Couldn't update flow.");
        return false;
      }
    },
    [projectId, id],
  );

  const handleRenameCommit = useCallback(async () => {
    if (!draft || draft.id !== id || !doc) return;
    const name = draft.name.trim();
    if (!name) {
      setDraft(null);
      return;
    }
    setDraft(null);
    if (name === meta?.name) return;
    const ok = await commitName(name, doc);
    if (ok) toast.success("Flow renamed.");
  }, [draft, id, doc, meta, commitName]);

  const handleServerSave = useCallback(
    async (next: FlowDoc) => {
      if (!projectId || !meta) return;
      const updated = await putFlow(
        projectId,
        id,
        draft?.id === id ? draft.name : meta.name,
        next,
      );
      setLoaded({ id, flow: updated });
    },
    [projectId, meta, draft, id],
  );

  const backTarget = "/flows";
  const backLabel = meta?.source === "schedule" ? "Back to schedules" : "Back to flows";
  const backHref = meta?.source === "schedule" ? "/schedules" : backTarget;

  if (!projectId) {
    return (
      <div className="flex min-h-full flex-1 flex-col">
        <div className="flex shrink-0 items-center gap-2 py-3">
          <Button variant="ghost" size="icon-sm" render={<Link href={backTarget} />}>
            <ArrowLeft className="size-4" />
            <span className="sr-only">Back to flows</span>
          </Button>
          <span className="text-sm font-semibold text-muted-foreground">
            Flow builder
          </span>
        </div>
        <div className="flex min-h-[50vh] flex-col items-center justify-center text-center">
          <h2 className="text-lg font-medium">No project selected</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Select a project to design its notification flows.
          </p>
          <Button variant="outline" size="sm" className="mt-4" render={<Link href={backTarget} />}>
            <ArrowLeft className="mr-1 size-3.5" /> Back to flows
          </Button>
        </div>
      </div>
    );
  }

  if (failure) {
    return (
      <div className="flex min-h-full flex-1 flex-col">
        <div className="flex shrink-0 items-center gap-2 py-3">
          <Button variant="ghost" size="icon-sm" render={<Link href={backTarget} />}>
            <ArrowLeft className="size-4" />
            <span className="sr-only">Back</span>
          </Button>
          <span className="text-sm font-semibold text-muted-foreground">
            Flow builder
          </span>
        </div>
        <div className="flex min-h-[50vh] flex-col items-center justify-center text-center">
          <h2 className="text-lg font-medium">
            {failure === "missing" ? "Flow not found" : "Couldn't load the flow"}
          </h2>
          <p className="mt-1 text-sm text-muted-foreground">
            {failure === "missing"
              ? "This flow doesn't exist in the current project."
              : "The server didn't respond — try again."}
          </p>
          <Button variant="outline" size="sm" className="mt-4" render={<Link href={backHref} />}>
            <ArrowLeft className="mr-1 size-3.5" /> {backLabel}
          </Button>
        </div>
      </div>
    );
  }

  if (!meta || !doc) {
    return (
      <div className="flex min-h-full flex-1 flex-col">
        <div className="flex shrink-0 items-center gap-2 py-3">
          <Button variant="ghost" size="icon-sm" render={<Link href={backTarget} />}>
            <ArrowLeft className="size-4" />
            <span className="sr-only">Back</span>
          </Button>
          <span className="text-sm font-semibold text-muted-foreground">
            Flow builder
          </span>
        </div>
        <div className="flex min-h-[50vh] flex-col items-center justify-center text-center">
          <h2 className="text-lg font-medium">Loading flow…</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Fetching the flow definition.
          </p>
        </div>
      </div>
    );
  }

  const ownerManaged = meta.source !== "flow_page";

  return (
    <div className="flex min-h-full flex-1 flex-col">
      <div className="relative flex min-h-[560px] min-w-0 flex-1 flex-col">
        {/* Top-center tab pill (n8n's Editor | Executions): Board keeps
            its full-bleed canvas; History swaps the content area while
            the board stays mounted underneath — unsaved edits, zoom,
            and open panels all survive the round-trip. */}
        <div className="absolute top-3 left-1/2 z-20 -translate-x-1/2">
          <div className="flex items-center gap-0.5 rounded-lg border border-border bg-popover/95 p-1 shadow-sm backdrop-blur dark:border-white/10 dark:bg-[#17171c]/95">
            {(
              [
                { id: "board", label: "Board", icon: GitBranch },
                { id: "history", label: "History", icon: History },
              ] as const
            ).map((t) => (
              <button
                key={t.id}
                type="button"
                aria-pressed={view === t.id}
                onClick={() => setView(t.id)}
                className={
                  "flex cursor-pointer items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors " +
                  (view === t.id
                    ? "bg-muted text-foreground dark:bg-white/10 dark:text-zinc-100"
                    : "text-muted-foreground hover:text-foreground dark:text-zinc-500 dark:hover:text-zinc-300")
                }
              >
                <t.icon className="size-3.5" /> {t.label}
              </button>
            ))}
          </div>
        </div>
        {/* The board stays mounted while History is open (display:none
            keeps React Flow's state alive). The wrapper is a flex
            container so the canvas root's own flex-1 keeps filling the
            full page height. */}
        <div className={view === "history" ? "hidden" : "flex min-h-0 flex-1"}>
          <FlowCanvas
          key={`${projectId}:${id}`}
          initialDoc={doc}
          onSave={handleServerSave}
          ownerManaged={ownerManaged}
          projectId={projectId}
          flowId={id}
          actions={
            <Button
              size="sm"
              variant="outline"
              aria-label={backLabel}
              title={backLabel}
              className="border-border bg-popover px-2 hover:bg-muted dark:border-white/15 dark:bg-[#17171c] dark:text-zinc-100 dark:hover:bg-white/10 dark:hover:text-white"
              render={<Link href={backHref} />}
            >
              <ArrowLeft className="size-3.5" />
            </Button>
          }
          topRight={
            <div className="absolute top-3 right-3 z-10 flex max-w-xs items-center gap-2 rounded-lg border border-border bg-popover/95 px-2 py-1 backdrop-blur dark:border-white/10 dark:bg-[#17171c]/95">
              <Input
                value={draft?.id === id ? draft.name : meta.name}
                onChange={(e) => setDraft({ id, name: e.target.value })}
                onBlur={() => void handleRenameCommit()}
                onKeyDown={(e) => {
                  if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                  if (e.key === "Escape") setDraft(null);
                }}
                aria-label="Flow name"
                className="h-6.5 w-40 border-transparent bg-transparent px-1.5 font-semibold hover:border-input focus-visible:border-ring dark:text-zinc-100 dark:hover:border-white/15 dark:focus-visible:border-white/25"
              />
              {ownerManaged ? (
                <Badge className="shrink-0 border-primary/20 bg-primary/10 text-[10px] font-medium text-primary">
                  Schedule flow
                </Badge>
              ) : (
                handlesSendEvent(meta.definition) && (
                  <Badge variant="secondary" className="shrink-0 text-[10px] font-medium">
                    Handles sends
                  </Badge>
                )
              )}
            </div>
          }
          />
        </div>
        {view === "history" && <FlowHistory projectId={projectId} flowId={id} />}
      </div>
    </div>
  );
}
