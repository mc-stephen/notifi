"use client";

import { cn } from "@/lib/utils";
import { Mail, Flame, Smartphone, MessageSquare, Globe, CheckCircle2, Plus, LucideIcon } from "lucide-react";
import type { ChannelConfig, Channel } from "@/lib/types";

const iconMap: Record<Channel, LucideIcon> = {
  email: Mail,
  fcm: Flame,
  apns: Smartphone,
  sms: MessageSquare,
  webpush: Globe,
};

const accentMap: Record<Channel, string> = {
  email: "from-blue-500/40 to-blue-500",
  fcm: "from-orange-500/40 to-orange-500",
  apns: "from-purple-500/40 to-purple-500",
  sms: "from-emerald-500/40 to-emerald-500",
  webpush: "from-cyan-500/40 to-cyan-500",
};

const bgMap: Record<Channel, string> = {
  email: "bg-blue-500/10 text-blue-400",
  fcm: "bg-orange-500/10 text-orange-400",
  apns: "bg-purple-500/10 text-purple-400",
  sms: "bg-emerald-500/10 text-emerald-400",
  webpush: "bg-cyan-500/10 text-cyan-400",
};

interface ChannelCardProps {
  channel: ChannelConfig;
  onConfigure: () => void;
}

export function ChannelCard({ channel, onConfigure }: ChannelCardProps) {
  const Icon = iconMap[channel.channel];

  return (
    <button onClick={onConfigure} className="w-full text-left group">
      <div
        className={cn(
          "card-lift relative overflow-hidden rounded-xl border border-border/40 bg-card",
          channel.configured && "ring-1 ring-emerald-500/20",
        )}
      >
        <div className={cn("h-1 w-full bg-gradient-to-r", accentMap[channel.channel])} />
        <div className="p-4">
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-3">
              <div className={cn("flex h-10 w-10 items-center justify-center rounded-lg", bgMap[channel.channel])}>
                {Icon && <Icon className="h-5 w-5" />}
              </div>
              <div>
                <p className="text-sm font-medium text-foreground">{channel.provider}</p>
                <p className="text-xs text-muted-foreground capitalize">{channel.channel}</p>
              </div>
            </div>
            {channel.configured ? (
              <CheckCircle2 className="h-4 w-4 text-emerald-500" />
            ) : (
              <Plus className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
            )}
          </div>
        </div>
      </div>
    </button>
  );
}
