"use client";

import { useCallback, useState } from "react";
import { toast } from "sonner";
import { createFlow, deleteFlow } from "@/hooks/use-flows";

/**
 * Lifecycle for flows created inline inside schedule panels ("New flow").
 *
 * The created flow is tracked as pending: saving the schedule commits it
 * (it becomes assigned and stays), while closing the panel unsaved
 * discards it (deleted, so half-made flows never pollute the Flows page).
 */
export function usePendingFlow(projectId: string | undefined, nameHint: string) {
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const create = useCallback(async (): Promise<{ id: string; name: string } | null> => {
    if (!projectId) return null;
    setBusy(true);
    try {
      const flow = await createFlow(projectId, nameHint);
      setPendingId(flow.id);
      return { id: flow.id, name: flow.name };
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't create flow.");
      return null;
    } finally {
      setBusy(false);
    }
  }, [projectId, nameHint]);

  /** The schedule was saved: the flow is assigned, stop tracking it. */
  const commit = useCallback(() => setPendingId(null), []);

  /** Panel closed unsaved: best-effort delete so no orphan remains. */
  const discard = useCallback(async () => {
    if (!projectId || !pendingId) return;
    setPendingId(null);
    try {
      await deleteFlow(projectId, pendingId);
    } catch {
      // Best effort — a leftover is visible on the Flows page, not fatal.
    }
  }, [projectId, pendingId]);

  /** Explicit trash action on the locked flow. True when deleted. */
  const removeLocked = useCallback(async (): Promise<boolean> => {
    if (!projectId || !pendingId) return false;
    setBusy(true);
    try {
      await deleteFlow(projectId, pendingId);
      setPendingId(null);
      return true;
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Couldn't delete flow.");
      return false;
    } finally {
      setBusy(false);
    }
  }, [projectId, pendingId]);

  return { pendingId, busy, create, commit, discard, removeLocked };
}
