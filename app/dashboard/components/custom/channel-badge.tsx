import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { NotificationChannel } from "@/lib/types";
import { CHANNEL_LABELS } from "@/lib/constants";
import {
  Mail,
  MessageSquare,
  Smartphone,
  Globe,
  Monitor,
  Apple,
  MessageCircle,
  Phone,
  Hash,
  Gamepad2,
  Send,
  Webhook,
} from "lucide-react";

export const CHANNEL_ICON_MAP: Record<NotificationChannel, React.ComponentType<{ className?: string }>> = {
  email: Mail,
  sms: MessageSquare,
  "push-android": Smartphone,
  "push-ios": Smartphone,
  "web-push": Globe,
  linux: Monitor,
  macos: Apple,
  rcs: MessageCircle,
  whatsapp: Phone,
  slack: Hash,
  discord: Gamepad2,
  teams: MessageSquare,
  telegram: Send,
  webhook: Webhook,
};

type ChannelBadgeProps = {
  channel: NotificationChannel;
  className?: string;
  showIcon?: boolean;
};

export function ChannelBadge({ channel, className, showIcon = true }: ChannelBadgeProps) {
  const Icon = CHANNEL_ICON_MAP[channel];

  return (
    <Badge variant="secondary" className={cn("gap-1.5 font-medium", className)}>
      {showIcon && <Icon className="size-3" />}
      {CHANNEL_LABELS[channel]}
    </Badge>
  );
}
