"use client";

import { useMemo, useState } from "react";
import {
  useProviderRegistry,
  useProviderConfigs,
  useProviderConfigActions,
  type ProviderConfig,
  type ProviderDefinition,
} from "@/hooks";
import { useProjectStore } from "@/store/project-store";
import { RECOMMENDED_PROVIDERS, providerBlurb } from "@/lib/constants";
import { ApiError } from "@/lib/api";
import { toast } from "sonner";
import { PageHeader } from "@/components/custom/page-header";
import { Button } from "@/components/ui/button";
import { ProviderCard } from "@/components/custom/providers/provider-card";
import { ProviderBrowsePanel } from "@/components/custom/providers/provider-browse-panel";
import { ProviderDetailPanel } from "@/components/custom/providers/provider-detail-panel";
import { ProviderConnectPanel } from "@/components/custom/providers/provider-connect-panel";
import {
  RecommendedIntegrations,
  type RecommendedItem,
} from "@/components/custom/providers/recommended-integrations";
import { cn } from "@/lib/utils";
import { Globe, Loader2 } from "lucide-react";

type Tab = "all" | "active" | "inactive";

const TABS: { id: Tab; label: string }[] = [
  { id: "all", label: "All" },
  { id: "active", label: "Active" },
  { id: "inactive", label: "Inactive" },
];

export type ConnectedProvider = ProviderConfig & {
  definition: ProviderDefinition;
  channelName: string;
  blurb: string;
};

function coerceValue(type: string, raw: unknown): unknown {
  if (type === "boolean") return raw === true || raw === "true";
  if (type === "number") {
    return raw === "" || raw === null || raw === undefined ? "" : Number(raw);
  }
  return raw === null || raw === undefined ? "" : String(raw);
}
export default function ProvidersPage() {
  const { registry, loading: registryLoading, error: registryError } =
    useProviderRegistry();
  const { configs, loading: configsLoading, error: configsError, refresh } =
    useProviderConfigs();
  const { test, create, update, remove } = useProviderConfigActions();
  const projectId = useProjectStore((s) => s.currentProject?.id);

  const [tab, setTab] = useState<Tab>("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [browseOpen, setBrowseOpen] = useState(false);
  const [browseSearch, setBrowseSearch] = useState("");
  const [connectDialog, setConnectDialog] = useState<{
    channel: string;
    provider: string;
  } | null>(null);
  /** Whether the open connect form came from browse (dismiss returns there). */
  const [connectFromBrowse, setConnectFromBrowse] = useState(false);
  const [configForm, setConfigForm] = useState<Record<string, unknown>>({});
  const [submitting, setSubmitting] = useState(false);

  const findDefinition = (
    channelId: string,
    providerId: string,
  ): { definition: ProviderDefinition; channelName: string } | null => {
    const channel = registry?.channels.find((c) => c.channel_id === channelId);
    const definition = channel?.providers.find(
      (p) => p.provider_id === providerId,
    );
    if (!channel || !definition) return null;
    return { definition, channelName: channel.channel_name };
  };

  const connected: ConnectedProvider[] = useMemo(() => {
    if (!registry) return [];
    return configs.flatMap((c) => {
      const found = findDefinition(c.channel_id, c.provider_id);
      if (!found) return [];
      return [
        {
          ...c,
          definition: found.definition,
          channelName: found.channelName,
          blurb: providerBlurb(c.channel_id, c.provider_id, found.channelName),
        },
      ];
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [configs, registry]);

  const recommended: RecommendedItem[] = useMemo(() => {
    if (!registry) return [];
    return RECOMMENDED_PROVIDERS.flatMap((r) => {
      const found = findDefinition(r.channel_id, r.provider_id);
      if (!found) return [];
      const existing = connected.find(
        (c) =>
          c.channel_id === r.channel_id && c.provider_id === r.provider_id,
      );
      return [
        {
          channelId: r.channel_id,
          definition: found.definition,
          channelName: found.channelName,
          blurb: providerBlurb(r.channel_id, r.provider_id, found.channelName),
          connected: !!existing,
          enabled: existing?.enabled ?? false,
          selected: existing ? existing.id === selectedId : false,
        },
      ];
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [registry, connected, selectedId]);

  const counts = useMemo(() => {
    const active = connected.filter((c) => c.enabled).length;
    return { all: connected.length, active, inactive: connected.length - active };
  }, [connected]);

  const visible = useMemo(() => {
    if (tab === "active") return connected.filter((c) => c.enabled);
    if (tab === "inactive") return connected.filter((c) => !c.enabled);
    return connected;
  }, [connected, tab]);

  const selected = useMemo(
    () => connected.find((c) => c.id === selectedId) ?? null,
    [connected, selectedId],
  );

  const toggleConfig = async (cfg: ConnectedProvider, enabled: boolean) => {
    try {
      await update(cfg.id, {
        channel_id: cfg.channel_id,
        provider_id: cfg.provider_id,
        config: cfg.config,
        enabled,
      });
      toast.success(
        enabled
          ? `${cfg.definition.name} enabled`
          : `${cfg.definition.name} disabled`,
      );
      refresh();
    } catch (e) {
      toast.error(e instanceof ApiError ? e.message : "Failed to update provider");
    }
  };

  const handleRecommendedAction = (item: RecommendedItem) => {
    const existing = connected.find(
      (c) =>
        c.channel_id === item.channelId &&
        c.provider_id === item.definition.provider_id,
    );
    if (existing) {
      setSelectedId((prev) => (prev === existing.id ? null : existing.id));
    } else {
      const found = findDefinition(item.channelId, item.definition.provider_id);
      if (!found) {
        toast.error("Couldn't load provider details. Please try again.");
        return;
      }
      setConfigForm({});
      setSelectedId(null);
      setBrowseOpen(false);
      setConnectFromBrowse(false);
      setConnectDialog({
        channel: item.channelId,
        provider: item.definition.provider_id,
      });
    }
  };

  const buildConnectPayload = (
    channel: string,
    provider: string,
    definition: ProviderDefinition,
  ) => {
    const coerced: Record<string, unknown> = {};
    for (const field of definition.config_fields) {
      coerced[field.key] = coerceValue(field.type, configForm[field.key]);
    }
    return {
      channel_id: channel,
      provider_id: provider,
      config: coerced,
      enabled: true,
    };
  };

  async function handleConnect() {
    if (!connectDialog || !registry) return;

    if (!projectId) {
      toast.error("No project selected. Please select or create a project first.");
      return;
    }
    const found = findDefinition(connectDialog.channel, connectDialog.provider);
    if (!found) return;

    for (const field of found.definition.config_fields) {
      if (!field.required) continue;
      const raw = configForm[field.key];
      const empty =
        raw === null ||
        raw === undefined ||
        (typeof raw === "string" && raw.trim() === "");
      if (empty) {
        toast.error(`${field.label} is required.`);
        return;
      }
    }
    setSubmitting(true);
    try {
      const payload = buildConnectPayload(
        connectDialog.channel,
        connectDialog.provider,
        found.definition,
      );
      const testResult = await test(
        payload.channel_id,
        payload.provider_id,
        payload.config,
      );

      if (!testResult.success) {
        toast.error(testResult.message);
        return;
      }
      const created = await create(payload);
      toast.success(testResult.message || "Provider connected successfully");
      setConnectDialog(null);
      setConfigForm({});
      setBrowseOpen(false);
      setConnectFromBrowse(false);
      setSelectedId(created.id);
      refresh();
    } catch (err) {
      toast.error(
        err instanceof ApiError ? err.message : "Failed to connect provider",
      );
    } finally {
      setSubmitting(false);
    }
  }
  function handleDialogClose() {
    setConnectDialog(null);
    setConfigForm({});
    if (connectFromBrowse) setBrowseOpen(true);
    setConnectFromBrowse(false);
  }
  if (registryLoading || configsLoading) {
    return (
      <div className="flex h-64 items-center justify-center">
        <Loader2 className="size-6 animate-spin text-muted-foreground" />
      </div>
    );
  }
  if (registryError || !registry) {
    return (
      <div className="flex h-64 items-center justify-center text-muted-foreground">
        Failed to load provider registry
      </div>
    );
  }
  const connectTarget = connectDialog
    ? findDefinition(connectDialog.channel, connectDialog.provider)
    : null;

  return (
    <div className="space-y-6">
      <PageHeader
        title="Providers"
        description={`${connected.length} connected provider${connected.length === 1 ? "" : "s"}`}
              />

      <RecommendedIntegrations
        items={recommended}
        onToggle={(item, enabled) => {
          const existing = connected.find(
            (c) =>
              c.channel_id === item.channelId &&
              c.provider_id === item.definition.provider_id,
          );
          if (existing) toggleConfig(existing, enabled);
        }}
        onAction={handleRecommendedAction}
      />

      <div className="space-y-4">
        <h2 className="text-lg font-semibold">Integration List</h2>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex gap-1 rounded-lg bg-muted p-1">
            {TABS.map((t) => (
              <button
                key={t.id}
                type="button"
                onClick={() => setTab(t.id)}
                className={cn(
                  "rounded-md px-4 py-1.5 text-sm transition-colors",
                  tab === t.id
                    ? "bg-background font-medium shadow"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {t.label}
              </button>
            ))}
          </div>
          <Button size="sm" className="gap-1.5" onClick={() => setBrowseOpen(true)}>
            <Globe className="size-3.5" /> Browse Integration
          </Button>
        </div>
      </div>

      {configsError && <p className="text-sm text-destructive">{configsError}</p>}
      {!projectId ? (
        <div className="rounded-xl border bg-card p-8 text-center text-sm text-muted-foreground">
          Select a project to manage providers.
        </div>
      ) : connected.length === 0 ? (
        <div className="rounded-xl border bg-card p-8 text-center">
          <p className="text-sm font-medium">No providers connected yet</p>
          <p className="mt-1 text-sm text-muted-foreground">
            Browse the catalog to connect your first provider.
          </p>
          <Button
            size="sm"
            className="mt-4 gap-1.5"
            onClick={() => setBrowseOpen(true)}
          >
            <Globe className="size-3.5" /> Browse Integration
          </Button>
        </div>
      ) : (
        <div className="flex min-h-0 flex-1 items-stretch">
          <div className="min-w-0 flex-1">
            {visible.length === 0 ? (
              <div className="rounded-xl border bg-card p-8 text-center text-sm text-muted-foreground">
                No providers in this view.
              </div>
            ) : (
              <div className="grid gap-4 md:grid-cols-2">
                {visible.map((cfg) => (
                  <ProviderCard
                    key={cfg.id}
                    name={cfg.definition.name}
                    blurb={cfg.blurb}
                    iconUrl={cfg.definition.icon_url}
                    docsUrl={cfg.definition.docs_url}
                    connected
                    enabled={cfg.enabled}
                    selected={cfg.id === selectedId}
                    onToggle={(enabled) => toggleConfig(cfg, enabled)}
                    onAction={() =>
                      setSelectedId((prev) =>
                        prev === cfg.id ? null : cfg.id,
                      )
                    }
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      )}
      {connectDialog && connectTarget ? (
        <ProviderConnectPanel
          key={`${connectDialog.channel}:${connectDialog.provider}`}
          definition={connectTarget.definition}
          channelName={connectTarget.channelName}
          blurb={providerBlurb(
            connectDialog.channel,
            connectDialog.provider,
            connectTarget.channelName,
          )}
          values={configForm}
          onChange={(key, value) =>
            setConfigForm((prev) => ({ ...prev, [key]: value }))
          }
          submitting={submitting}
          canSubmit={!!projectId}
          onSubmit={handleConnect}
          onClose={handleDialogClose}
        />
      ) : (
        selected && (
          <ProviderDetailPanel
            key={selected.id}
            config={selected}
            definition={selected.definition}
            channelName={selected.channelName}
            blurb={selected.blurb}
            test={test}
            update={update}
            remove={remove}
            onChanged={refresh}
            onDeleted={() => {
              setSelectedId(null);
              refresh();
            }}
            onClose={() => setSelectedId(null)}
          />
        )
      )}
      {browseOpen && registry && (
        <ProviderBrowsePanel
          channels={registry.channels}
          isConnected={(channelId, providerId) =>
            connected.some(
              (c) =>
                c.channel_id === channelId &&
                c.provider_id === providerId,
            )
          }
          search={browseSearch}
          onSearchChange={setBrowseSearch}
          blurbFor={providerBlurb}
          onConnect={(channelId, providerId) => {
            const found = findDefinition(channelId, providerId);
            if (!found) {
              toast.error("Couldn't load provider details. Please try again.");
              return;
            }
            setConfigForm({});
            setSelectedId(null);
            setBrowseOpen(false);
            setConnectFromBrowse(true);
            setConnectDialog({ channel: channelId, provider: providerId });
          }}
          onClose={() => setBrowseOpen(false)}
        />
      )}

    </div>
  );
}
