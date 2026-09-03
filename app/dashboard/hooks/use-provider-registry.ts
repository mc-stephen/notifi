import { useState, useEffect } from "react";
import { api } from "@/lib/api";

export interface ConfigField {
  key: string;
  label: string;
  type: "text" | "password" | "email" | "number" | "boolean";
  required: boolean;
}

export interface SmtpFallbackConfig {
  fields: ConfigField[];
}

export interface ProviderDefinition {
  provider_id: string;
  name: string;
  scope: "global" | "regional";
  primary_regions?: string[];
  platforms?: string[];
  icon_url?: string;
  config_fields: ConfigField[];
  smtp_fallback?: SmtpFallbackConfig;
}

export interface ChannelDefinition {
  channel_id: string;
  channel_name: string;
  providers: ProviderDefinition[];
}

export interface ProviderRegistry {
  version: string;
  last_updated: string;
  channels: ChannelDefinition[];
}

export function useProviderRegistry() {
  const [registry, setRegistry] = useState<ProviderRegistry | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function fetchRegistry() {
      try {
        const data = await api<ProviderRegistry>("/v1/providers");
        setRegistry(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error");
      } finally {
        setLoading(false);
      }
    }

    fetchRegistry();
  }, []);

  return { registry, loading, error };
}
