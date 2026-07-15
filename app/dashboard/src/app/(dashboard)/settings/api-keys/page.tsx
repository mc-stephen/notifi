"use client";

import { useSettings } from "@/hooks/use-settings";
import { PageHeader } from "@/components/dashboard/shared/page-header";
import { ApiKeysManager } from "@/components/dashboard/settings/api-keys-manager";
import { IdempotencyIndicator } from "@/components/dashboard/settings/idempotency-indicator";

export default function ApiKeysPage() {
  const { keys, showKeys, toggleKeyVisibility, rollKey } = useSettings();

  return (
    <div className="mx-auto w-full max-w-7xl space-y-6 p-5">
      <PageHeader
        title="API Keys"
        description="Manage your developer API keys and credentials."
      />

      <IdempotencyIndicator />

      <ApiKeysManager
        keys={keys}
        showKeys={showKeys}
        onToggleVisibility={toggleKeyVisibility}
        onRoll={rollKey}
      />
    </div>
  );
}
