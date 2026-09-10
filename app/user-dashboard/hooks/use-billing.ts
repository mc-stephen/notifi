"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";
import type { BillingCycle, BillingPlan, ProjectSubscription } from "@/lib/types";

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
 * Project-scoped billing: plan catalog + current subscription.
 * New projects land on free — the API ensures it on first read.
 */
export function useBilling() {
  const projectId = useProjectStore((s) => s.currentProject?.id);

  const [state, setState] = useState<BillingState>({
    plans: [],
    subscription: null,
    loading: true,
    error: null,
  });

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
  }, [projectId]);

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
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  const subscribe = useCallback(
    async (planId: string, billingCycle: BillingCycle) => {
      if (!projectId) throw new Error("Select a project first");
      const res = await api<{ subscription: ProjectSubscription }>("/app/billing/subscriptions", {
        method: "POST",
        body: JSON.stringify({ projectId, planId, billingCycle }),
      });
      setState((prev) => ({ ...prev, subscription: res.subscription }));
      return res.subscription;
    },
    [projectId]
  );

  const cancel = useCallback(async () => {
    if (!projectId) throw new Error("Select a project first");
    const res = await api<{ subscription: ProjectSubscription }>("/app/billing/subscriptions/cancel", {
      method: "POST",
      body: JSON.stringify({ projectId }),
    });
    setState((prev) => ({ ...prev, subscription: res.subscription }));
    return res.subscription;
  }, [projectId]);

  return {
    plans: state.plans,
    subscription: state.subscription,
    loading: state.loading,
    error: state.error,
    refresh: load,
    subscribe,
    cancel,
  };
}
