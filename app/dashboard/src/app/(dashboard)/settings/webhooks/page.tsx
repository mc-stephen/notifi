"use client";

import { useSettings } from "@/hooks/use-settings";
import { PageHeader } from "@/components/dashboard/shared/page-header";
import { WebhooksConfigurator } from "@/components/dashboard/settings/webhooks-configurator";

export default function WebhooksPage() {
  const { webhooks, availableEvents, addWebhook, removeWebhook } = useSettings();

  return (
    <div className="mx-auto w-full max-w-7xl space-y-6 p-5">
      <PageHeader
        title="Webhooks"
        description="Configure external webhook endpoints for event notifications."
      />

      <WebhooksConfigurator
        webhooks={webhooks}
        availableEvents={availableEvents}
        onAdd={addWebhook}
        onRemove={removeWebhook}
      />
    </div>
  );
}
