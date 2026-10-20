"use client";

import { Button } from "@/components/ui/button";
import { Loader2 } from "lucide-react";
import type { ProviderDefinition } from "@/hooks/use-provider-registry";
import { DetailPanel } from "@/components/custom/detail-panel";
import { ProviderConfigFields } from "./provider-config-fields";
import { ProviderIcon } from "./provider-icon";

type ProviderConnectPanelProps = {
  definition: ProviderDefinition;
  channelName: string;
  blurb: string;
  values: Record<string, unknown>;
  onChange: (key: string, value: unknown) => void;
  submitting: boolean;
  /** False when no project is selected (submit disabled). */
  canSubmit: boolean;
  onSubmit: () => void;
  onClose: () => void;
};

/**
 * Right slide-over for connecting a new provider: credential form for a
 * not-yet-connected definition. Renders in the shared DetailPanel shell;
 * on success the caller swaps it for the settings panel of the created
 * config.
 */
export function ProviderConnectPanel({
  definition,
  channelName,
  blurb,
  values,
  onChange,
  submitting,
  canSubmit,
  onSubmit,
  onClose,
}: ProviderConnectPanelProps) {
  return (
    <DetailPanel
      onClose={onClose}
      closeLabel="Close connect panel"
      header={
        <div className="flex items-center gap-3">
          <ProviderIcon name={definition.name} iconUrl={definition.icon_url} />
          <div className="min-w-0">
            <h3 className="truncate font-semibold text-foreground">
              Connect {definition.name}
            </h3>
            <p className="truncate text-xs text-muted-foreground">{blurb}</p>
          </div>
        </div>
      }
    >
      {/* Body */}
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="space-y-5 px-5 py-4">
          <p className="text-xs text-muted-foreground">
            Configure your API credentials to connect this provider to{" "}
            {channelName}. Only connected providers appear in your
            integration list.
          </p>
          <ProviderConfigFields
            definition={definition}
            values={values}
            onChange={onChange}
          />
          {definition.docs_url && (
            <a
              href={definition.docs_url}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-primary hover:underline"
            >
              Find your credentials in the {definition.name} docs
            </a>
          )}
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center gap-2 border-t border-border bg-muted/30 px-5 py-3">
        <Button
          variant="outline"
          onClick={onClose}
          disabled={submitting}
        >
          Cancel
        </Button>
        <Button
          className="flex-1"
          onClick={onSubmit}
          disabled={submitting || !canSubmit}
        >
          {submitting ? (
            <>
              <Loader2 className="size-4 animate-spin" /> Connecting...
            </>
          ) : (
            "Connect"
          )}
        </Button>
      </div>
    </DetailPanel>
  );
}
