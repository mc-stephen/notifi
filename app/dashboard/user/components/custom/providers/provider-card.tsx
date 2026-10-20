"use client";

import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { ExternalLink, Settings } from "lucide-react";
import { ProviderIcon } from "./provider-icon";

type ProviderCardProps = {
  name: string;
  blurb: string;
  iconUrl?: string;
  docsUrl?: string;
  /** Whether this provider is connected to the project. */
  connected: boolean;
  enabled: boolean;
  selected?: boolean;
  toggleDisabled?: boolean;
  actionLabel?: string;
  /** Slimmer density + no shadow for the Recommended strip. */
  compact?: boolean;
  /** Extra classes for the card root (e.g. surface override per usage). */
  className?: string;
  onToggle?: (enabled: boolean) => void;
  onAction?: () => void;
};

export function ProviderCard({
  name,
  blurb,
  iconUrl,
  docsUrl,
  connected,
  enabled,
  selected,
  toggleDisabled,
  actionLabel,
  compact,
  className,
  onToggle,
  onAction,
}: ProviderCardProps) {
  return (
    <div
      className={cn(
        "flex flex-col rounded-xl border bg-card text-card-foreground transition-colors",
        compact ? "p-3" : "p-4",
        selected && "border-primary",
        className,
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <ProviderIcon name={name} iconUrl={iconUrl} size={compact ? "sm" : "md"} />
        <Switch
          checked={connected && enabled}
          disabled={toggleDisabled ?? !connected}
          onCheckedChange={onToggle}
          aria-label={`${enabled ? "Disable" : "Enable"} ${name}`}
        />
      </div>
      <p className={cn("font-semibold", compact ? "mt-2 text-[13px]" : "mt-3 text-sm")}>{name}</p>
      <p
        title={blurb}
        className={cn(
          "text-muted-foreground",
          compact ? "mt-0.5 line-clamp-1 text-xs" : "mt-0.5 text-sm",
        )}
      >
        {blurb}
      </p>
      <div className={cn("flex items-center justify-between gap-2", compact ? "mt-2" : "mt-3")}>
        <Button size="sm" variant="outline" className="gap-1.5" onClick={onAction}>
          <Settings className="size-3.5" />
          {actionLabel ?? (connected ? "Settings" : "Connect")}
        </Button>
        {docsUrl && (
          <a
            href={docsUrl}
            target="_blank"
            rel="noopener noreferrer"
            aria-label={`${name} documentation`}
            className="rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
            onClick={(e) => e.stopPropagation()}
          >
            <ExternalLink className="size-4" />
          </a>
        )}
      </div>
    </div>
  );
}
