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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ProviderCard } from "@/components/custom/providers/provider-card";
import { ProviderIcon } from "@/components/custom/providers/provider-icon";
import { ProviderDetailPanel } from "@/components/custom/providers/provider-detail-panel";
import {
  RecommendedIntegrations,
  type RecommendedItem,
} from "@/components/custom/providers/recommended-integrations";
import { cn } from "@/lib/utils";
import { ExternalLink, Globe, Loader2, Plug, Search } from "lucide-react";

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
function toText(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "boolean") return value ? "true" : "false";
  return String(value);
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
      setConfigForm({});
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
  const searchLower = browseSearch.toLowerCase();
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
        <div className="relative flex min-h-0 flex-1 items-stretch gap-6">
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

          {selected && (
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
          )}
        </div>
      )}
      {/* Browse catalog */}
      <Dialog open={browseOpen} onOpenChange={setBrowseOpen}>
        <DialogContent className="flex max-h-[80vh] flex-col overflow-hidden sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>Browse integrations</DialogTitle>
            <DialogDescription>
              Connect a provider to this project. Only connected providers
              appear in your integration list.
            </DialogDescription>
          </DialogHeader>
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={browseSearch}
              placeholder="Search providers..."
              onChange={(e) => setBrowseSearch(e.target.value)}
              className="h-9 pl-8 text-sm"
            />
          </div>
          <div className="min-h-0 flex-1 space-y-6 overflow-y-auto pr-1">
            {registry.channels.map((channel) => {
              const providers = channel.providers.filter((p) =>
                p.name.toLowerCase().includes(searchLower),
              );
              if (providers.length === 0) return null;
              return (
                <div key={channel.channel_id} className="space-y-2">
                  <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    {channel.channel_name}
                  </p>
                  <div className="space-y-2">
                    {providers.map((provider) => {
                      const isConnected = connected.some(
                        (c) =>
                          c.channel_id === channel.channel_id &&
                          c.provider_id === provider.provider_id,
                      );
                      return (
                        <div
                          key={provider.provider_id}
                          className="flex items-center gap-3 rounded-lg border p-3"
                        >
                          <ProviderIcon
                            name={provider.name}
                            iconUrl={provider.icon_url}
                          />
                          <div className="min-w-0 flex-1">
                            <p className="truncate text-sm font-medium">
                              {provider.name}
                            </p>
                            <p className="truncate text-xs text-muted-foreground">
                              {providerBlurb(
                                channel.channel_id,
                                provider.provider_id,
                                channel.channel_name,
                              )}
                            </p>
                          </div>
                          {provider.docs_url && (
                            <a
                              href={provider.docs_url}
                              target="_blank"
                              rel="noopener noreferrer"
                              aria-label={`${provider.name} documentation`}
                              className="shrink-0 rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
                              onClick={(e) => e.stopPropagation()}
                            >
                              <ExternalLink className="size-4" />
                            </a>
                          )}
                          {isConnected ? (
                            <Badge variant="secondary" className="text-[10px]">
                              Connected
                            </Badge>
                          ) : (
                            <Button
                              size="sm"
                              variant="outline"
                              className="gap-1.5"
                              onClick={() => {
                                setConfigForm({});
                                setConnectDialog({
                                  channel: channel.channel_id,
                                  provider: provider.provider_id,
                                });
                              }}
                            >
                              <Plug className="size-3" /> Connect
                            </Button>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>
        </DialogContent>
      </Dialog>

      {/* Connect dialog */}
      <Dialog open={!!connectDialog} onOpenChange={handleDialogClose}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Connect provider</DialogTitle>
            <DialogDescription>
              Configure your API credentials to connect this provider. Only
              connected providers appear in your integration list.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            {connectTarget?.definition.config_fields.map((field) => {
              if (field.type === "boolean") {
                return (
                  <div key={field.key} className="flex items-center gap-2">
                    <Checkbox
                      checked={configForm[field.key] === true}
                      onCheckedChange={(v) =>
                        setConfigForm({ ...configForm, [field.key]: v === true })
                      }
                    />
                    <Label className="text-sm font-medium">
                      {field.label}
                      {field.required && (
                        <span className="ml-1 text-destructive">*</span>
                      )}
                    </Label>
                  </div>
                );
              }
              return (
                <div key={field.key} className="space-y-2">
                  <Label className="text-sm font-medium">
                    {field.label}
                    {field.required && (
                      <span className="ml-1 text-destructive">*</span>
                    )}
                  </Label>
                  <Input
                    placeholder={`Enter ${field.label.toLowerCase()}`}
                    type={
                      field.type === "password"
                        ? "password"
                        : field.type === "email"
                          ? "email"
                          : field.type === "number"
                            ? "number"
                            : "text"
                    }
                    value={toText(configForm[field.key])}
                    onChange={(e) =>
                      setConfigForm({
                        ...configForm,
                        [field.key]: e.target.value,
                      })
                    }
                  />
                </div>
              );
            })}
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={handleDialogClose}
              disabled={submitting}
            >
              Cancel
            </Button>
            <Button onClick={handleConnect} disabled={submitting || !projectId}>
              {submitting ? (
                <>
                  <Loader2 className="size-4 animate-spin" /> Connecting...
                </>
              ) : (
                "Connect"
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
