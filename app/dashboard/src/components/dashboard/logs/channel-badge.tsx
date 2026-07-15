"use client";

import { cn } from "@/lib/utils";
import type { Channel } from "@/lib/types";
import { CHANNEL_LABELS } from "@/lib/constants";
import { Mail, Flame, Smartphone, MessageSquare, Globe } from "lucide-react";

const iconMap: Record<Channel, React.ElementType> = {
  email: Mail,
  fcm: Flame,
  apns: Smartphone,
  sms: MessageSquare,
  webpush: Globe,
};

const channelStyles: Record<Channel, string> = {
  email: "bg-blue-500/10 text-blue-500 border-blue-500/20",
  fcm: "bg-orange-500/10 text-orange-500 border-orange-500/20",
  apns: "bg-purple-500/10 text-purple-500 border-purple-500/20",
  sms: "bg-emerald-500/10 text-emerald-500 border-emerald-500/20",
  webpush: "bg-cyan-500/10 text-cyan-500 border-cyan-500/20",
};

interface ChannelBadgeProps {
  channel: Channel;
}

export function ChannelBadge({ channel }: ChannelBadgeProps) {
  const Icon = iconMap[channel];

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium",
        channelStyles[channel],
      )}
    >
      {Icon && <Icon className="h-3 w-3" />}
      {CHANNEL_LABELS[channel]}
    </span>
  );
}
