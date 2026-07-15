"use client";

import { Eye, EyeOff, RotateCcw, Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { CopyButton } from "@/components/dashboard/shared/copy-button";
import type { ApiKey } from "@/lib/types";

interface ApiKeyCardProps {
  key_: ApiKey;
  visible: boolean;
  onToggleVisibility: () => void;
  onRoll: () => void;
}

export function ApiKeyCard({ key_, visible, onToggleVisibility, onRoll }: ApiKeyCardProps) {
  return (
    <div className="card-lift rounded-xl border border-border/40 bg-card p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Badge variant={key_.environment === "production" ? "default" : "secondary"}>
            {key_.environment}
          </Badge>
          <span className="text-xs text-muted-foreground">
            Created {new Date(key_.createdAt).toLocaleDateString()}
          </span>
        </div>
        <Button variant="ghost" size="icon-sm" onClick={onRoll} title="Roll key">
          <RotateCcw className="h-3.5 w-3.5" />
        </Button>
      </div>

      <div className="flex items-center gap-2 rounded-lg bg-secondary/80 px-3 py-2.5 font-mono text-sm border border-border/40">
        <span className="flex-1 truncate">
          {visible ? key_.maskedKey.replace(/x/g, "•") : key_.maskedKey}
        </span>
        <button
          onClick={onToggleVisibility}
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          {visible ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
        </button>
        <CopyButton value={key_.maskedKey} />
      </div>
    </div>
  );
}
