"use client";

import { useState } from "react";
import { DetailPanel } from "@/components/custom/detail-panel";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ApiError } from "@/lib/api";
import type { ProviderDefinition } from "@/hooks/use-provider-registry";
import type {
  ProviderConfig,
  ProviderConfigInput,
} from "@/hooks/use-provider-configs";
import {
  Check,
  ExternalLink,
  Loader2,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";
import { ProviderIcon } from "./provider-icon";
import { ProviderConfigFields } from "./provider-config-fields";

type ProviderDetailPanelProps = {
  config: ProviderConfig;
  definition: ProviderDefinition;
  channelName: string;
  blurb: string;
  test: (
    channel_id: string,
    provider_id: string,
    config: Record<string, unknown>,
  ) => Promise<{ success: boolean; message: string }>;
  update: (configId: string, input: ProviderConfigInput) => Promise<unknown>;
  remove: (configId: string) => Promise<void>;
  onChanged: () => void;
  onDeleted: () => void;
  onClose: () => void;
};

export function ProviderDetailPanel({
  config,
  definition,
  channelName,
  blurb,
  test,
  update,
  remove,
  onChanged,
  onDeleted,
  onClose,
}: ProviderDetailPanelProps) {
  const [values, setValues] = useState<Record<string, unknown>>({
    ...(config.config ?? {}),
  });
  const [verified, setVerified] = useState(false);
  const [saving, setSaving] = useState(false);
  const [toggling, setToggling] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const setValue = (key: string, value: unknown) => {
    setValues((prev) => ({ ...prev, [key]: value }));
    setVerified(false);
    setFormError(null);
  };

  const buildPayload = (enabled: boolean): ProviderConfigInput => {
    const coerced: Record<string, unknown> = {};
    for (const field of definition.config_fields) {
      const raw = values[field.key];
      if (field.type === "boolean") {
        coerced[field.key] = raw === true || raw === "true";
      } else if (field.type === "number") {
        coerced[field.key] =
          raw === "" || raw === null || raw === undefined ? "" : Number(raw);
      } else {
        coerced[field.key] = raw === null || raw === undefined ? "" : String(raw);
      }
    }
    return {
      channel_id: config.channel_id,
      provider_id: config.provider_id,
      config: coerced,
      enabled,
    };
  };

  const handleToggle = async (enabled: boolean) => {
    setToggling(true);
    try {
      await update(config.id, buildPayload(enabled));
      toast.success(
        enabled
          ? `${definition.name} enabled`
          : `${definition.name} disabled`,
      );
      onChanged();
    } catch (e) {
      toast.error(e instanceof ApiError ? e.message : "Failed to update provider");
    } finally {
      setToggling(false);
    }
  };

  const handleSave = async () => {
    for (const field of definition.config_fields) {
      if (!field.required) continue;
      const raw = values[field.key];
      const empty =
        raw === null ||
        raw === undefined ||
        (typeof raw === "string" && raw.trim() === "");
      if (empty) {
        setFormError(`${field.label} is required.`);
        return;
      }
    }

    setSaving(true);
    setFormError(null);
    try {
      const payload = buildPayload(config.enabled);
      const result = await test(
        payload.channel_id,
        payload.provider_id,
        payload.config,
      );
      if (!result.success) {
        toast.error(result.message);
        return;
      }
      await update(config.id, payload);
      setVerified(true);
      toast.success(result.message || "Provider settings saved");
      onChanged();
    } catch (e) {
      toast.error(e instanceof ApiError ? e.message : "Failed to save settings");
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    setDeleting(true);
    try {
      await remove(config.id);
      toast.success(`${definition.name} disconnected`);
      setDeleteOpen(false);
      onDeleted();
    } catch (e) {
      toast.error(
        e instanceof ApiError ? e.message : "Failed to disconnect provider",
      );
    } finally {
      setDeleting(false);
    }
  };

  return (
    <DetailPanel
      onClose={onClose}
      closeLabel="Close settings"
      headerClassName="items-center"
      header={
        <h3 className="font-semibold text-foreground">Integration Settings</h3>
      }
    >

      {/* Scrollable body */}
      <div className="min-h-0 flex-1 overflow-y-auto">
        {/* Provider identity */}
        <div className="flex items-center gap-3 px-5 py-4">
          <ProviderIcon name={definition.name} iconUrl={definition.icon_url} />
          <div className="min-w-0 flex-1">
            <p className="truncate text-sm font-semibold">{definition.name}</p>
            <p className="truncate text-xs text-muted-foreground">{blurb}</p>
          </div>
          <Switch
            checked={config.enabled}
            disabled={toggling}
            onCheckedChange={handleToggle}
            aria-label={`${config.enabled ? "Disable" : "Enable"} ${definition.name}`}
          />
        </div>

        <div className="space-y-5 px-5 py-4">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-semibold">Integration Details</h4>
            {verified && (
              <Badge
                variant="outline"
                className="gap-1 border-success/30 bg-success/10 text-xs font-medium text-success"
              >
                <Check className="size-3" /> Verified
              </Badge>
            )}
          </div>

          {formError && <p className="text-xs text-destructive">{formError}</p>}

          <ProviderConfigFields
            definition={definition}
            values={values}
            onChange={setValue}
          />

          {definition.smtp_fallback && (
            <p className="text-xs text-muted-foreground">
              SMTP fallback available for this provider.
            </p>
          )}

          {definition.docs_url && (
            <a
              href={definition.docs_url}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1.5 text-xs text-primary hover:underline"
            >
              Find your credentials in the {definition.name} docs
              <ExternalLink className="size-3" />
            </a>
          )}

          <p className="text-xs text-muted-foreground">
            Channel: {channelName} · Connected{" "}
            {new Date(config.created_at).toLocaleDateString(undefined, {
              month: "short",
              day: "numeric",
              year: "numeric",
            })}
          </p>
        </div>
      </div>

      {/* Sticky footer */}
      <div className="flex items-center justify-between gap-3 border-t px-5 py-3">
        <Button
          size="sm"
          variant="outline"
          className="gap-1.5 text-destructive hover:text-destructive"
          onClick={() => setDeleteOpen(true)}
        >
          <Trash2 className="size-3.5" /> Disconnect
        </Button>
        <Button size="sm" disabled={saving} onClick={handleSave}>
          {saving && <Loader2 className="size-3.5 animate-spin" />}
          Save changes
        </Button>
      </div>

      <Dialog open={deleteOpen} onOpenChange={setDeleteOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Disconnect provider</DialogTitle>
            <DialogDescription>
              Disconnect {definition.name} from this project? Channels routing
              through it will stop delivering.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteOpen(false)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              disabled={deleting}
              onClick={handleDelete}
            >
              {deleting && <Loader2 className="size-3.5 animate-spin" />}
              Disconnect
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </DetailPanel>
  );
}
