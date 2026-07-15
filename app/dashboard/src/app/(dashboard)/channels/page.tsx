"use client";

import { useChannels } from "@/hooks/use-channels";
import { PageHeader } from "@/components/dashboard/shared/page-header";
import { ChannelGrid } from "@/components/dashboard/channels/channel-grid";
import { ChannelConfigModal } from "@/components/dashboard/channels/channel-config-modal";
import { EmptyState } from "@/components/dashboard/shared/empty-state";
import { PlugZap } from "lucide-react";

export default function ChannelsPage() {
  const {
    channels,
    selectedChannel,
    configuring,
    startConfiguring,
    saveConfig,
    cancelConfig,
  } = useChannels();

  return (
    <div className="mx-auto w-full max-w-7xl space-y-6 p-5">
      <PageHeader
        title="Channels & Integrations"
        description="Connect and configure third-party notification providers."
      />

      {channels.length === 0 ? (
        <EmptyState
          icon={<PlugZap className="h-6 w-6" />}
          title="No channels configured"
          description="Connect your first notification provider to start sending messages."
        />
      ) : (
        <ChannelGrid channels={channels} onConfigure={startConfiguring} />
      )}

      <ChannelConfigModal
        channel={selectedChannel}
        open={configuring}
        onSave={saveConfig}
        onCancel={cancelConfig}
      />
    </div>
  );
}
