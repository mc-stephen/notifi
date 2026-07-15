"use client";

import { type LucideIcon } from "lucide-react";
import type { ChannelConfig } from "@/lib/types";
import { ChannelCard } from "./channel-card";

interface ChannelCategorySectionProps {
  title: string;
  icon: LucideIcon;
  channels: ChannelConfig[];
  onConfigure: (channel: ChannelConfig) => void;
}

export function ChannelCategorySection({
  title,
  icon: Icon,
  channels,
  onConfigure,
}: ChannelCategorySectionProps) {
  if (channels.length === 0) return null;

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        {Icon && (
          <div className="flex h-6 w-6 items-center justify-center rounded-md bg-secondary">
            <Icon className="h-3.5 w-3.5 text-muted-foreground" />
          </div>
        )}
        <h3 className="text-sm font-medium text-foreground">{title}</h3>
        <span className="text-xs text-muted-foreground">({channels.length})</span>
      </div>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {channels.map((ch) => (
          <ChannelCard
            key={ch.id}
            channel={ch}
            onConfigure={() => onConfigure(ch)}
          />
        ))}
      </div>
    </div>
  );
}
