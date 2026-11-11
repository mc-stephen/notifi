"use client";

import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { InboundProbeDto } from "@/components/custom/flows/flow-data";

/** The webhook door's last authorized ping (`GET
 * /app/flows/{id}/inbound/last`) — the builder's "Incoming data" tab
 * for webhook triggers. Fetches once when enabled; a failure or an
 * unpinged door both read as "no probe". State settles only from the
 * fetch callbacks (no synchronous setState in the effect). */
export function useInboundProbe(
  enabled: boolean,
  projectId?: string,
  flowId?: string,
  token?: string,
): { loading: boolean; probe: InboundProbeDto | null } {
  const [result, setResult] = useState<{ probe: InboundProbeDto | null } | null>(null);

  useEffect(() => {
    if (!enabled || !projectId || !flowId) return;
    let alive = true;
    const qs = new URLSearchParams({ project_id: projectId });
    if (token) qs.set("token", token);
    api<{ probe: InboundProbeDto | null }>(`/app/flows/${flowId}/inbound/last?${qs}`)
      .then((res) => {
        if (alive) setResult({ probe: res.probe });
      })
      .catch(() => {
        if (alive) setResult({ probe: null });
      });
    return () => {
      alive = false;
    };
  }, [enabled, projectId, flowId, token]);

  return { loading: enabled && result === null, probe: result?.probe ?? null };
}
