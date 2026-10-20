"use client";

import type { ProviderDefinition } from "@/hooks/use-provider-registry";
import { ProviderCard } from "./provider-card";

export type RecommendedItem = {
  channelId: string;
  definition: ProviderDefinition;
  channelName: string;
  blurb: string;
  connected: boolean;
  enabled: boolean;
  selected: boolean;
};

type RecommendedIntegrationsProps = {
  items: RecommendedItem[];
  onToggle: (item: RecommendedItem, enabled: boolean) => void;
  onAction: (item: RecommendedItem) => void;
};

export function RecommendedIntegrations({
  items,
  onToggle,
  onAction,
}: RecommendedIntegrationsProps) {
  if (items.length === 0) return null;

  return (
    <section className="rounded-xl border bg-card p-4 text-card-foreground">
      <h2 className="text-sm font-semibold">Recommended Integration Apps</h2>
      <p className="mt-0.5 text-xs text-muted-foreground">
        Enhance your notification delivery by connecting these essential
        providers for seamless operations.
      </p>
      <div className="mt-3 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {items.map((item) => (
          <ProviderCard
            key={`${item.channelId}:${item.definition.provider_id}`}
            name={item.definition.name}
            blurb={item.blurb}
            iconUrl={item.definition.icon_url}
            docsUrl={item.definition.docs_url}
            connected={item.connected}
            enabled={item.enabled}
            selected={item.selected}
            compact
            className="bg-popover"
            onToggle={(enabled) => onToggle(item, enabled)}
            onAction={() => onAction(item)}
          />
        ))}
      </div>
    </section>
  );
}
