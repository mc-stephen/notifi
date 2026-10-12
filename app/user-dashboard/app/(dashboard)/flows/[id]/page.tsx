"use client";

import { use, useCallback, useState } from "react";
import Link from "next/link";
import { ArrowLeft } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { FlowCanvas } from "@/components/custom/flows/FlowCanvas";
import { useProjectStore } from "@/store/project-store";
import {
  asFlowDoc,
  handlesSendEvent,
  putFlow,
  useFlows,
} from "@/hooks/use-flows";
import type { FlowDoc } from "@/components/custom/flows/flow-types";

export default function FlowBuilderPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const { flows, loading, refresh } = useFlows();
  const meta = flows.find((m) => m.id === id) ?? null;
  const doc = meta ? asFlowDoc(meta.definition) : null;
  // Null draft follows the stored name; typing takes over until committed.
  const [draft, setDraft] = useState<string | null>(null);

  const commitRename = useCallback(async () => {
    if (!projectId || draft === null || !meta || !doc) return;
    const name = draft.trim();
    setDraft(null);
    if (!name || name === meta.name) return;
    try {
      await putFlow(projectId, id, name, doc);
      await refresh();
      toast.success("Flow renamed.");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't rename flow.");
    }
  }, [projectId, draft, meta, doc, id, refresh]);

  const handleServerSave = useCallback(
    async (next: FlowDoc) => {
      if (!projectId || !meta) return;
      const name = draft ?? meta.name;
      await putFlow(projectId, id, name, next);
    },
    [projectId, meta, draft, id],
  );

  if (!projectId) {
    return (
      <div className="flex min-h-full flex-1 flex-col">
        <div className="flex shrink-0 items-center gap-2 py-3">
          <Button variant="ghost" size="icon-sm" render={<Link href="/flows" />}>
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
          <Button variant="outline" size="sm" className="mt-4" render={<Link href="/flows" />}>
            <ArrowLeft className="mr-1 size-3.5" /> Back to flows
          </Button>
        </div>
      </div>
    );
  }

  if (!loading && !meta) {
    return (
      <div className="flex min-h-full flex-1 flex-col">
        <div className="flex shrink-0 items-center gap-2 py-3">
          <Button variant="ghost" size="icon-sm" render={<Link href="/flows" />}>
            <ArrowLeft className="size-4" />
            <span className="sr-only">Back to flows</span>
          </Button>
          <span className="text-sm font-semibold text-muted-foreground">
            Flow builder
          </span>
        </div>
        <div className="flex min-h-[50vh] flex-col items-center justify-center text-center">
          <h2 className="text-lg font-medium">Flow not found</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            This flow does not exist in the current project.
          </p>
          <Button variant="outline" size="sm" className="mt-4" render={<Link href="/flows" />}>
            <ArrowLeft className="mr-1 size-3.5" /> Back to flows
          </Button>
        </div>
      </div>
    );
  }

  if (!meta || !doc) {
    return (
      <div className="flex min-h-full flex-1 flex-col">
        <div className="flex shrink-0 items-center gap-2 py-3">
          <Button variant="ghost" size="icon-sm" render={<Link href="/flows" />}>
            <ArrowLeft className="size-4" />
            <span className="sr-only">Back to flows</span>
          </Button>
          <span className="text-sm font-semibold text-muted-foreground">
            Flow builder
          </span>
        </div>
        <div className="flex min-h-[50vh] flex-col items-center justify-center text-center">
          <h2 className="text-lg font-medium">
            {loading ? "Loading flow…" : "Flow data is corrupt"}
          </h2>
          <p className="mt-1 text-sm text-muted-foreground">
            {loading
              ? "Fetching the flow definition."
              : "The stored definition isn't a valid flow document."}
          </p>
          <Button variant="outline" size="sm" className="mt-4" render={<Link href="/flows" />}>
            <ArrowLeft className="mr-1 size-3.5" /> Back to flows
          </Button>
        </div>
      </div>
    );
  }

  const sends = handlesSendEvent(meta.definition);

  return (
    <div className="flex min-h-full flex-1 flex-col">
      <div className="relative flex min-h-[560px] min-w-0 flex-1 flex-col">
        <FlowCanvas
          key={`${projectId}:${id}`}
          initialDoc={doc}
          onSave={handleServerSave}
          actions={
            <Button
              size="sm"
              variant="outline"
              aria-label="Back to flows"
              title="Back to flows"
              className="border-border bg-popover px-2 hover:bg-muted dark:border-white/15 dark:bg-[#17171c] dark:text-zinc-100 dark:hover:bg-white/10 dark:hover:text-white"
              render={<Link href="/flows" />}
            >
              <ArrowLeft className="size-3.5" />
            </Button>
          }
        />
        <div className="absolute top-3 right-3 z-10 flex max-w-xs items-center gap-2 rounded-lg border border-border bg-popover/95 px-3 py-1.5 shadow-xl backdrop-blur dark:border-white/10 dark:bg-[#17171c]/95">
          <Input
            value={draft ?? meta.name}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => void commitRename()}
            onKeyDown={(e) => {
              if (e.key === "Enter") (e.target as HTMLInputElement).blur();
              if (e.key === "Escape") setDraft(null);
            }}
            aria-label="Flow name"
            className="h-7 border-transparent bg-transparent font-semibold hover:border-input focus-visible:border-ring dark:text-zinc-100 dark:hover:border-white/15 dark:focus-visible:border-white/25"
          />
          {sends && (
            <Badge variant="secondary" className="shrink-0 text-[10px] font-medium">
              Handles sends
            </Badge>
          )}
        </div>
      </div>
    </div>
  );
}
