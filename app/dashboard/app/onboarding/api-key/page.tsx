"use client";

import { useEffect, useState } from "react";
import { Copy, Check, Eye, EyeOff, KeyRound } from "lucide-react";
import { Button } from "@/components/ui/button";
import { OnboardingNav } from "@/components/custom/onboarding/onboarding-nav";
import { useOnboardingStore } from "@/store/onboarding-store";

function generateMockKey(): string {
  const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
  let key = "nft_live_";
  for (let i = 0; i < 48; i++) {
    key += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return key;
}

export default function ApiKeyPage() {
  const { apiKey, apiKeyGenerated, updateData } = useOnboardingStore();
  const [key, setKey] = useState(apiKey || "");
  const [revealed, setRevealed] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!apiKeyGenerated) {
      const mockKey = generateMockKey();
      setKey(mockKey);
      updateData({ apiKey: mockKey, apiKeyGenerated: true });
    }
  }, [apiKeyGenerated, updateData, apiKey]);

  const maskedKey = key
    ? key.substring(0, 12) + "••••••••••••••••••••" + key.slice(-4)
    : "";

  const handleCopy = async () => {
    if (key) {
      await navigator.clipboard.writeText(key);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="flex flex-col">
      <div className="mb-6 flex items-center gap-3">
        <div className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <KeyRound className="size-5" />
        </div>
        <div>
          <h1 className="text-2xl font-bold tracking-tight">
            Generate your API key
          </h1>
          <p className="text-sm text-muted-foreground">
            Use this key to authenticate requests to the Notifi API.
          </p>
        </div>
      </div>

      <div className="mb-10 space-y-6">
        {/* API Key display */}
        <div className="rounded-xl border bg-muted/30 p-4">
          <div className="mb-3 flex items-center justify-between">
            <span className="text-xs font-medium text-muted-foreground">
              Your API Key
            </span>
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2"
                onClick={() => setRevealed(!revealed)}
              >
                {revealed ? (
                  <EyeOff className="size-3" />
                ) : (
                  <Eye className="size-3" />
                )}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2"
                onClick={handleCopy}
              >
                {copied ? (
                  <Check className="size-3 text-green-500" />
                ) : (
                  <Copy className="size-3" />
                )}
              </Button>
            </div>
          </div>
          <code className="block break-all font-mono text-sm">
            {revealed ? key : maskedKey}
          </code>
        </div>

        {/* Security warning */}
        <div className="rounded-xl border border-amber-500/20 bg-amber-500/5 p-4">
          <div className="mb-1 text-sm font-medium text-amber-600 dark:text-amber-400">
            Keep this key secret
          </div>
          <p className="text-xs text-muted-foreground">
            Never expose your API key in client-side code or public repositories.
            Store it securely in an environment variable.
          </p>
        </div>

        {/* Quick code snippet */}
        <div>
          <div className="mb-2 text-xs font-medium text-muted-foreground">
            Quick start
          </div>
          <div className="overflow-x-auto rounded-xl border bg-muted/30 p-4">
            <pre className="text-xs">
              <code>
                {`curl -X POST https://api.notifi.dev/v1/notifications \\
  -H "Authorization: Bearer ${revealed ? key : "nft_live_••••••••••••••••"}" \\
  -H "Content-Type: application/json" \\
  -d '{"recipient": "user@example.com", "template": "welcome"}'`}
              </code>
            </pre>
          </div>
        </div>
      </div>

      <OnboardingNav showSkip nextLabel="Continue" />
    </div>
  );
}
