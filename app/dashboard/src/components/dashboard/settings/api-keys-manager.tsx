"use client";

import { useState } from "react";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ApiKeyCard } from "./api-key-card";
import type { ApiKey } from "@/lib/types";

interface ApiKeysManagerProps {
  keys: ApiKey[];
  showKeys: Record<string, boolean>;
  onToggleVisibility: (id: string) => void;
  onRoll: (id: string) => void;
}

export function ApiKeysManager({
  keys,
  showKeys,
  onToggleVisibility,
  onRoll,
}: ApiKeysManagerProps) {
  const [generating, setGenerating] = useState(false);

  const handleGenerate = async () => {
    setGenerating(true);
    await new Promise((r) => setTimeout(r, 600));
    setGenerating(false);
    alert("New API key generated (mock)");
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-muted-foreground">
          Keys are used to authenticate API requests. Keep them secure.
        </p>
        <Button size="sm" onClick={handleGenerate} disabled={generating}>
          <Plus className="h-3.5 w-3.5" />
          Generate Key
        </Button>
      </div>

      <div className="space-y-3">
        {keys.map((key_) => (
          <ApiKeyCard
            key={key_.id}
            key_={key_}
            visible={!!showKeys[key_.id]}
            onToggleVisibility={() => onToggleVisibility(key_.id)}
            onRoll={() => onRoll(key_.id)}
          />
        ))}
      </div>

      {keys.length === 0 && (
        <p className="text-sm text-muted-foreground text-center py-8">
          No API keys yet. Generate your first key above.
        </p>
      )}
    </div>
  );
}
