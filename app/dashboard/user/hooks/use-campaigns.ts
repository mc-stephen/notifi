"use client";

import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useProjectStore } from "@/store/project-store";

export type CampaignStatus =
  | "draft"
  | "scheduled"
  | "running"
  | "paused"
  | "cancelled"
  | "completed"
  | "failed";

export type AudienceMode = "all" | "custom";

export type CampaignTotals = {
  /** Expanded rows — the recipients contacted by this campaign. */
  total: number;
  delivered: number;
  failed: number;
  pending: number;
};

export type ServerCampaign = {
  id: string;
  projectId: string;
  name: string;
  description: string;
  createdBy: string;
  channel: string;
  /** The template the campaign renders from (provenance). */
  templateId: string | null;
  /** The frozen copy snapshot — rendered at compose time, read-only. */
  title: string;
  body: string;
  /** Who gets it: "all" or "custom" (a compose-time selection). */
  audienceMode: AudienceMode;
  /** Custom-mode targeting keys (recipient `userId`s). */
  selectedUserIds: string[];
  /** Explicit primary provider pick ("" = auto: first connected). */
  providerId: string;
  status: CampaignStatus;
  /** Scheduled fire time (waiting for `running` to start). */
  sendAt: string | null;
  startedAt: string | null;
  completedAt: string | null;
  /** Why a campaign failed (empty audience, storage failure, ...). */
  lastError: string | null;
  createdAt: string;
  updatedAt: string;
};

export type ServerCampaignWithStats = ServerCampaign & {
  totals: CampaignTotals;
  /** Fire-time audience estimate for campaigns that haven't expanded. */
  estimatedAudience: number | null;
};

export type CampaignCreateInput = {
  name: string;
  description?: string;
  /** Campaigns compose from templates — required by the server. */
  templateId: string;
  /** Which template version (channel) to send — required by the server. */
  channel?: string;
  /** The connected provider to try first; omitted lets the server
      order the chain (the rest stay fallbacks). */
  providerId?: string;
  audienceMode?: AudienceMode;
  selectedUserIds?: string[];
  /** Some when scheduling the fire; omitted for a plain draft. */
  sendAt?: string | null;
};

export type CampaignPatchInput = {
  name?: string;
  description?: string;
  /** A different template re-renders the snapshot. */
  templateId?: string;
  /** Re-picks which template channel to send through. */
  channel?: string;
  /** Re-picks the primary provider ("": clear to auto order). */
  providerId?: string;
  audienceMode?: AudienceMode;
  selectedUserIds?: string[];
  /** null (unschedule) turns a scheduled campaign back into a draft. */
  sendAt?: string | null;
};

function toMessage(e: unknown, fallback: string) {
  return e instanceof Error ? e.message : fallback;
}

export async function createCampaign(
  projectId: string,
  input: CampaignCreateInput,
): Promise<ServerCampaignWithStats> {
  const res = await api<{ campaign: ServerCampaignWithStats }>(
    "/app/campaigns",
    {
      method: "POST",
      body: JSON.stringify({ projectId, ...input }),
    },
  );
  return res.campaign;
}

export async function patchCampaign(
  projectId: string,
  id: string,
  patch: CampaignPatchInput,
): Promise<ServerCampaignWithStats> {
  const res = await api<{ campaign: ServerCampaignWithStats }>(
    `/app/campaigns/${id}`,
    { method: "PATCH", body: JSON.stringify({ projectId, ...patch }) },
  );
  return res.campaign;
}

export async function deleteCampaign(
  projectId: string,
  id: string,
): Promise<boolean> {
  const res = await api<{ deleted: boolean }>(
    `/app/campaigns/${id}?project_id=${encodeURIComponent(projectId)}`,
    { method: "DELETE" },
  );
  return res.deleted;
}

/** Bulk fire: expands the audience, drains the first batch inline. */
export async function campaignAction(
  projectId: string,
  id: string,
  action: "run-now" | "pause" | "resume" | "cancel",
): Promise<ServerCampaignWithStats> {
  const res = await api<{ campaign: ServerCampaignWithStats }>(
    `/app/campaigns/${id}/${action}`,
    { method: "POST", body: JSON.stringify({ projectId }) },
  );
  return res.campaign;
}

/** Project campaigns with derived totals, server-side source of truth. */
export function useCampaigns() {
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [campaigns, setCampaigns] = useState<ServerCampaignWithStats[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!projectId) {
      setCampaigns([]);
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const res = await api<{ campaigns: ServerCampaignWithStats[] }>(
        `/app/campaigns?project_id=${encodeURIComponent(projectId)}`,
      );
      setCampaigns(res.campaigns ?? []);
    } catch (e) {
      setCampaigns([]);
      setError(toMessage(e, "Failed to load campaigns"));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    let ignore = false;
    (async () => {
      if (!projectId) {
        if (!ignore) {
          setCampaigns([]);
          setLoading(false);
        }
        return;
      }
      try {
        const res = await api<{ campaigns: ServerCampaignWithStats[] }>(
          `/app/campaigns?project_id=${encodeURIComponent(projectId)}`,
        );
        if (!ignore) setCampaigns(res.campaigns ?? []);
      } catch (e) {
        if (!ignore) {
          setCampaigns([]);
          setError(toMessage(e, "Failed to load campaigns"));
        }
      } finally {
        if (!ignore) setLoading(false);
      }
    })();
    return () => {
      ignore = true;
    };
  }, [projectId]);

  return { campaigns, loading, error, refresh };
}
