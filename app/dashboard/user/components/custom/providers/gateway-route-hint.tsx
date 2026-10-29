"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Check, Copy } from "lucide-react";
import { env } from "@/lib/env";
import { gatewaySubscribeUrl } from "@/lib/gateway";

type GatewayRouteHintProps = {
  topic: string;
  secretKey: string;
};

/**
 * The copyable subscribe line for a connected Notifi gateway — the exact
 * URL the user's Linux app polls (append `&wait=20` to long-poll). Shows
 * only once both route halves are present.
 */
export function GatewayRouteHint({ topic, secretKey }: GatewayRouteHintProps) {
  const [copied, setCopied] = useState(false);
  const trimmed = topic.trim();
  const key = secretKey.trim();
  if (!trimmed || !key) return null;

  const url = gatewaySubscribeUrl(env.apiBase, trimmed, key);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      setCopied(false);
    }
  };

  return (
    <div className="space-y-1.5 rounded-lg border border-border bg-muted/40 px-3 py-2.5">
      <p className="text-xs font-medium">Gateway subscription</p>
      <p className="text-xs text-muted-foreground">
        Your Linux app polls this route; append <code>&amp;wait=20</code> to
        long-poll so messages land instantly.
      </p>
      <div className="flex items-center gap-1.5">
        <code className="min-w-0 flex-1 truncate font-mono text-[11px]">
          {url}
        </code>
        <Button
          variant="ghost"
          size="icon-xs"
          className="shrink-0"
          onClick={handleCopy}
          aria-label="Copy subscribe URL"
        >
          {copied ? <Check className="size-3 text-success" /> : <Copy className="size-3" />}
        </Button>
      </div>
    </div>
  );
}
