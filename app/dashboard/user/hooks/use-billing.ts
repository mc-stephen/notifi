"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";
import type { BillingCycle, BillingPlan, ProjectSubscription, Receipt } from "@/lib/types";

type BillingState = {
  plans: BillingPlan[];
  subscription: ProjectSubscription | null;
  loading: boolean;
  error: string | null;
};

function toMessage(e: unknown, fallback: string) {
  return e instanceof Error ? e.message : fallback;
}

/**
 * Project-scoped billing: plan catalog + current subscription + receipt
 * history. New projects land on free — the API ensures it on first read.
 * Receipts load quietly: history failure never blocks plans/subscription.
 */
export function useBilling() {
  const projectId = useProjectStore((s) => s.currentProject?.id);

  const [state, setState] = useState<BillingState>({
    plans: [],
    subscription: null,
    loading: true,
    error: null,
  });
  const [receipts, setReceipts] = useState<Receipt[]>([]);

  const loadReceipts = useCallback(async () => {
    if (!projectId) {
      setReceipts([]);
      return;
    }
    try {
      const res = await api<{ receipts: Receipt[] }>(
        `/app/billing/history?project_id=${encodeURIComponent(projectId)}`
      );
      setReceipts(res.receipts ?? []);
    } catch {
      setReceipts([]);
    }
  }, [projectId]);

  const load = useCallback(async () => {
    setState((prev) => ({ ...prev, loading: true, error: null }));
    try {
      const { plans } = await api<{ plans: BillingPlan[] }>("/app/billing/plans");
      let subscription: ProjectSubscription | null = null;
      if (projectId) {
        const res = await api<{ subscription: ProjectSubscription }>(
          `/app/billing/subscription?project_id=${encodeURIComponent(projectId)}`
        );
        subscription = res.subscription;
      }
      setState({ plans, subscription, loading: false, error: null });
    } catch (e) {
      setState((prev) => ({ ...prev, loading: false, error: toMessage(e, "Failed to load billing") }));
    }
    await loadReceipts();
  }, [projectId, loadReceipts]);

  useEffect(() => {
    let ignore = false;
    (async () => {
      setState((prev) => ({ ...prev, loading: true, error: null }));
      try {
        const { plans } = await api<{ plans: BillingPlan[] }>("/app/billing/plans");
        let subscription: ProjectSubscription | null = null;
        if (projectId) {
          const res = await api<{ subscription: ProjectSubscription }>(
            `/app/billing/subscription?project_id=${encodeURIComponent(projectId)}`
          );
          subscription = res.subscription;
        }
        if (ignore) return;
        setState({ plans, subscription, loading: false, error: null });
      } catch (e) {
        if (ignore) return;
        setState((prev) => ({ ...prev, loading: false, error: toMessage(e, "Failed to load billing") }));
      }
      if (ignore) return;
      await loadReceipts();
    })();
    return () => {
      ignore = true;
    };
  }, [projectId, loadReceipts]);

  const subscribe = useCallback(
    async (planId: string, billingCycle: BillingCycle) => {
      if (!projectId) throw new Error("Select a project first");
      const res = await api<{ subscription: ProjectSubscription }>("/app/billing/subscriptions", {
        method: "POST",
        body: JSON.stringify({ projectId, planId, billingCycle }),
      });
      setState((prev) => ({ ...prev, subscription: res.subscription }));
      await loadReceipts();
      return res.subscription;
    },
    [projectId, loadReceipts]
  );

  const cancel = useCallback(async () => {
    if (!projectId) throw new Error("Select a project first");
    const res = await api<{ subscription: ProjectSubscription }>("/app/billing/subscriptions/cancel", {
      method: "POST",
      body: JSON.stringify({ projectId }),
    });
    setState((prev) => ({ ...prev, subscription: res.subscription }));
    await loadReceipts();
    return res.subscription;
  }, [projectId, loadReceipts]);

  return {
    plans: state.plans,
    subscription: state.subscription,
    receipts,
    loading: state.loading,
    error: state.error,
    refresh: load,
    subscribe,
    cancel,
  };
}
