"use client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ExternalLink, Plug, Search } from "lucide-react";
import type {
  ChannelDefinition,
  ProviderDefinition,
} from "@/hooks/use-provider-registry";
import { DetailPanel } from "@/components/custom/detail-panel";
import { ProviderIcon } from "./provider-icon";

type ProviderBrowsePanelProps = {
  channels: ChannelDefinition[];
  isConnected: (channelId: string, providerId: string) => boolean;
  search: string;
  onSearchChange: (value: string) => void;
  blurbFor: (
    channelId: string,
    providerId: string,
    channelName: string,
  ) => string;
  onConnect: (channelId: string, providerId: string) => void;
  onClose: () => void;
};

/**
 * Right slide-over variant of the integration catalog: searchable,
 * channel-grouped provider list. Same content as the old centered browse
 * dialog, rendered in the shared DetailPanel shell. Picking Connect hands
 * off to the caller's connect flow (centered popup).
 */
export function ProviderBrowsePanel({
  channels,
  isConnected,
  search,
  onSearchChange,
  blurbFor,
  onConnect,
  onClose,
}: ProviderBrowsePanelProps) {
  const searchLower = search.toLowerCase();

  return (
    <DetailPanel
      onClose={onClose}
      closeLabel="Close browse panel"
      className="w-[min(30rem,94%)]"
      header={
        <div>
          <h3 className="font-semibold text-foreground">
            Browse integrations
          </h3>
          <p className="mt-0.5 text-xs text-muted-foreground">
            Connect a provider to this project. Only connected providers
            appear in your integration list.
          </p>
        </div>
      }
    >
      <div className="shrink-0 border-b border-border px-5 py-3">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={search}
            placeholder="Search providers..."
            onChange={(e) => onSearchChange(e.target.value)}
            className="h-9 pl-8 text-sm"
          />
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="space-y-4 px-5 py-4">
          {channels.map((channel) => {
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
                  {providers.map((provider: ProviderDefinition) => (
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
                          {blurbFor(
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
                      {isConnected(
                        channel.channel_id,
                        provider.provider_id,
                      ) ? (
                        <Badge variant="secondary" className="text-[10px]">
                          Connected
                        </Badge>
                      ) : (
                        <Button
                          size="sm"
                          variant="outline"
                          className="gap-1.5"
                          onClick={() =>
                            onConnect(channel.channel_id, provider.provider_id)
                          }
                        >
                          <Plug className="size-3" /> Connect
                        </Button>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </DetailPanel>
  );
}
