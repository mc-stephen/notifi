"use client";

import { useState } from "react";
import { useProviderRegistry } from "@/hooks";
import { useProjectStore } from "@/store/project-store";
import { api, ApiError } from "@/lib/api";
import { toast } from "sonner";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Plug, Loader2, Search, ChevronDown, ExternalLink } from "lucide-react";

const CHANNEL_ICONS: Record<string, string> = {
  email: "✉",
  sms: "💬",
  push: "🔔",
  chat: "💬",
};

export default function ProvidersPage() {
  const { registry, loading, error } = useProviderRegistry();
  const projectId = useProjectStore((s) => s.currentProject?.id);
  const [search, setSearch] = useState("");
  const [selectedChannel, setSelectedChannel] = useState<string | null>(null);
  const [connectDialog, setConnectDialog] = useState<{
    channel: string;
    provider: string;
  } | null>(null);
  const [configForm, setConfigForm] = useState<Record<string, string>>({});
  const [submitting, setSubmitting] = useState(false);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="size-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error || !registry) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        Failed to load provider registry
      </div>
    );
  }

  const channels = registry.channels;
  const allProviders = channels.flatMap((c) =>
    c.providers.map((p) => ({
      ...p,
      channel_id: c.channel_id,
      channel_name: c.channel_name,
    })),
  );

  const searchLower = search.toLowerCase();
  const hasSearch = searchLower.length > 0;
  const hasChannelFilter = selectedChannel !== null;

  async function handleConnect() {
    if (!connectDialog) return;

    if (!projectId) {
      toast.error(
        "No project selected. Please select or create a project first.",
      );
      return;
    }

    setSubmitting(true);
    try {
      // First, test the connection
      const testResult = await api<{ success: boolean; message: string }>(
        `/v1/projects/${projectId}/channel-configs/test`,
        {
          method: "POST",
          body: JSON.stringify({
            channel_id: connectDialog.channel,
            provider_id: connectDialog.provider,
            config: configForm,
          }),
        }
      );

      if (!testResult.success) {
        toast.error(testResult.message);
        return;
      }

      // If test passes, save the config
      await api(`/v1/projects/${projectId}/channel-configs`, {
        method: "POST",
        body: JSON.stringify({
          channel_id: connectDialog.channel,
          provider_id: connectDialog.provider,
          config: configForm,
          enabled: true,
        }),
      });
      toast.success(testResult.message || "Provider connected successfully");
      setConnectDialog(null);
      setConfigForm({});
    } catch (err) {
      const message =
        err instanceof ApiError ? err.message : "Failed to connect provider";
      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  }

  function handleDialogClose() {
    setConnectDialog(null);
    setConfigForm({});
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Providers"
        description={`${allProviders.length} available providers across ${channels.length} channels`}
        breadcrumbs={[
          { label: "Dashboard", href: "/" },
          { label: "Providers" },
        ]}
        actions={
          <div className="flex items-center gap-2">
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
              <Input
                value={search}
                placeholder="Search providers..."
                onChange={(e) => setSearch(e.target.value)}
                className="h-9 w-64 pl-8 text-sm"
              />
            </div>
            <div className="relative">
              <select
                value={selectedChannel ?? ""}
                onChange={(e) => setSelectedChannel(e.target.value || null)}
                className="h-9 appearance-none rounded-md border bg-transparent px-3 pr-8 text-sm"
              >
                <option value="">All Channels</option>
                {channels.map((c) => (
                  <option key={c.channel_id} value={c.channel_id}>
                    {c.channel_name}
                  </option>
                ))}
              </select>
              <ChevronDown className="absolute right-2 top-1/2 -translate-y-1/2 size-4 text-muted-foreground pointer-events-none" />
            </div>
          </div>
        }
      />

      <div className="space-y-8">
        {channels
          .filter(
            (channel) =>
              !hasChannelFilter || channel.channel_id === selectedChannel,
          )
          .map((channel) => {
            const providers = channel.providers.filter((p) => {
              if (hasSearch && !p.name.toLowerCase().includes(searchLower))
                return false;
              return true;
            });

            if (providers.length === 0) return null;

            return (
              <div key={channel.channel_id} className="space-y-3">
                <div className="flex items-center gap-2">
                  <span className="text-lg">
                    {CHANNEL_ICONS[channel.channel_id] ?? "📦"}
                  </span>
                  <h3 className="text-sm font-semibold">
                    {channel.channel_name}
                  </h3>
                  <Badge variant="secondary" className="text-[10px]">
                    {providers.length}
                  </Badge>
                </div>
                <div className="grid gap-3 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
                  {providers.map((provider) => (
                    <Card
                      key={`${channel.channel_id}-${provider.provider_id}`}
                      className="hover:bg-muted/30 transition-colors flex flex-col"
                    >
                      <CardHeader className="pb-2">
                        <div className="flex items-start justify-between">
                          <div className="flex items-center gap-2">
                            {provider.icon_url ? (
                              <img
                                src={provider.icon_url}
                                alt={provider.name}
                                className="size-8 rounded-lg object-contain bg-white border"
                                onError={(e) => {
                                  const target = e.target as HTMLImageElement;
                                  target.style.display = "none";
                                  target.nextElementSibling?.classList.remove(
                                    "hidden",
                                  );
                                }}
                              />
                            ) : null}
                            <div
                              className={`flex size-8 items-center justify-center rounded-lg bg-muted text-xs font-bold ${provider.icon_url ? "hidden" : ""}`}
                            >
                              {provider.name.charAt(0)}
                            </div>
                            <CardTitle className="text-sm">
                              {provider.name}
                            </CardTitle>
                            {provider.docs_url && (
                              <a
                                href={provider.docs_url}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-muted-foreground hover:text-foreground ml-1"
                                onClick={(e) => e.stopPropagation()}
                              >
                                <ExternalLink className="size-3" />
                              </a>
                            )}
                          </div>
                        </div>
                      </CardHeader>
                      <CardContent className="space-y-2 flex-1 flex flex-col">
                        <div className="flex flex-wrap gap-1">
                          {provider.config_fields.slice(0, 3).map((field) => (
                            <Badge
                              key={field.key}
                              variant="outline"
                              className="text-[10px]"
                            >
                              {field.label}
                            </Badge>
                          ))}
                          {provider.config_fields.length > 3 && (
                            <Badge variant="outline" className="text-[10px]">
                              +{provider.config_fields.length - 3} more
                            </Badge>
                          )}
                        </div>
                        <div className="min-h-[20px]">
                          {provider.smtp_fallback && (
                            <Badge
                              variant="secondary"
                              className="text-[10px] bg-warning/10 text-warning"
                            >
                              SMTP Fallback Available
                            </Badge>
                          )}
                        </div>
                        <Button
                          size="sm"
                          variant="outline"
                          className="w-full gap-1.5 mt-auto"
                          onClick={() =>
                            setConnectDialog({
                              channel: channel.channel_id,
                              provider: provider.provider_id,
                            })
                          }
                        >
                          <Plug className="size-3" /> Connect
                        </Button>
                      </CardContent>
                    </Card>
                  ))}
                </div>
              </div>
            );
          })}
        {hasSearch &&
          channels.every((c) =>
            c.providers.every(
              (p) => !p.name.toLowerCase().includes(searchLower),
            ),
          ) && (
            <div className="py-12 text-center text-sm text-muted-foreground">
              No providers match your search.
            </div>
          )}
      </div>

      <Dialog open={!!connectDialog} onOpenChange={handleDialogClose}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Connect provider</DialogTitle>
            <DialogDescription>
              Configure your API credentials to connect this provider. You can
              then assign it to a channel.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            {connectDialog &&
              (() => {
                const channel = channels.find(
                  (c) => c.channel_id === connectDialog.channel,
                );
                const provider = channel?.providers.find(
                  (p) => p.provider_id === connectDialog.provider,
                );
                if (!provider) return null;
                return provider.config_fields.map((field) => (
                  <div key={field.key} className="space-y-2">
                    <label className="text-sm font-medium">
                      {field.label}
                      {field.required && (
                        <span className="text-destructive ml-1">*</span>
                      )}
                    </label>
                    <Input
                      placeholder={`Enter ${field.label.toLowerCase()}`}
                      type={
                        field.type === "password"
                          ? "password"
                          : field.type === "email"
                            ? "email"
                            : "text"
                      }
                      value={configForm[field.key] ?? ""}
                      onChange={(e) =>
                        setConfigForm({
                          ...configForm,
                          [field.key]: e.target.value,
                        })
                      }
                    />
                  </div>
                ));
              })()}
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
