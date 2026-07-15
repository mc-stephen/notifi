"use client";

import { Mail, Flame, MessageSquare, Globe } from "lucide-react";
import { ChannelCategorySection } from "./channel-category-section";
import type { ChannelConfig } from "@/lib/types";

interface ChannelGridProps {
  channels: ChannelConfig[];
  onConfigure: (channel: ChannelConfig) => void;
}

export function ChannelGrid({ channels, onConfigure }: ChannelGridProps) {
  const emailChannels = channels.filter((c) => c.channel === "email");
  const pushChannels = channels.filter((c) => c.channel === "fcm" || c.channel === "apns");
  const smsChannels = channels.filter((c) => c.channel === "sms");
  const webChannels = channels.filter((c) => c.channel === "webpush");

  return (
    <div className="space-y-8">
      <ChannelCategorySection
        title="Email"
        icon={Mail}
        channels={emailChannels}
        onConfigure={onConfigure}
      />
      <ChannelCategorySection
        title="Mobile Push"
        icon={Flame}
        channels={pushChannels}
        onConfigure={onConfigure}
      />
      <ChannelCategorySection
        title="SMS / RCS"
        icon={MessageSquare}
        channels={smsChannels}
        onConfigure={onConfigure}
      />
      <ChannelCategorySection
        title="Web & Desktop"
        icon={Globe}
        channels={webChannels}
        onConfigure={onConfigure}
      />
    </div>
  );
}
