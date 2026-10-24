"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { cn } from "@/lib/utils";
import {
  Mail,
  Smartphone,
  Globe,
  Webhook,
  Check,
  Monitor,
  Apple,
  Terminal,
  Bell,
} from "lucide-react";
import { OnboardingNav } from "@/components/custom/onboarding/onboarding-nav";
import { useOnboardingStore } from "@/store/onboarding-store";

const CHANNELS = [
  {
    id: "email",
    label: "Email",
    description: "Send transactional and marketing emails via SMTP or providers",
    icon: Mail,
    configured: true,
  },
  {
    id: "sms",
    label: "SMS",
    description: "Text messages via Twilio, Vonage, or SNS",
    icon: Smartphone,
    configured: false,
  },
  {
    id: "window",
    label: "Windows",
    description: "Desktop toasts via WNS or a self-hosted webhook",
    icon: Monitor,
    configured: false,
  },
  {
    id: "android",
    label: "Android",
    description: "Push to Android devices via FCM, Pushy, or OneSignal",
    icon: Smartphone,
    configured: false,
  },
  {
    id: "apple",
    label: "Apple",
    description: "Push to iOS and macOS via APNs or OneSignal",
    icon: Apple,
    configured: false,
  },
  {
    id: "linux",
    label: "Linux",
    description: "Desktop notifications via a self-hosted endpoint",
    icon: Terminal,
    configured: false,
  },
  {
    id: "web",
    label: "Web Push",
    description: "Browser push via the VAPID protocol",
    icon: Bell,
    configured: false,
  },
  {
    id: "in_app",
    label: "In-App",
    description: "Real-time notifications in your application via WebSocket",
    icon: Globe,
    configured: true,
  },
  {
    id: "webhook",
    label: "Webhooks",
    description: "HTTP callbacks to your endpoints",
    icon: Webhook,
    configured: false,
  },
];

export default function ChannelsPage() {
  const router = useRouter();
  const { selectedChannels, updateData } = useOnboardingStore();
  const [selected, setSelected] = useState<string[]>(selectedChannels);

  const toggleChannel = (id: string) => {
    setSelected((prev) =>
      prev.includes(id) ? prev.filter((c) => c !== id) : [...prev, id]
    );
  };

  const handleContinue = () => {
    updateData({ selectedChannels: selected });
    useOnboardingStore.getState().nextStep();
    router.push(
      useOnboardingStore
        .getState()
        .getStepRoute(useOnboardingStore.getState().currentStep)
    );
  };

  return (
    <div className="flex flex-col">
      <h1 className="mb-2 text-2xl font-bold tracking-tight">
        Configure channels
      </h1>
      <p className="mb-8 text-muted-foreground">
        Select which notification channels to set up. You can configure these
        later in Settings.
      </p>

      <div className="mb-10 space-y-3">
        {CHANNELS.map((ch) => {
          const Icon = ch.icon;
          const isSelected = selected.includes(ch.id);
          return (
            <button
              key={ch.id}
              onClick={() => toggleChannel(ch.id)}
              className={cn(
                "flex w-full items-center gap-4 rounded-xl border p-4 text-left transition-colors hover:bg-muted/50",
                isSelected && "border-primary bg-primary/5 ring-1 ring-primary"
              )}
            >
              <div
                className={cn(
                  "flex size-10 shrink-0 items-center justify-center rounded-lg",
                  isSelected
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted text-muted-foreground"
                )}
              >
                <Icon className="size-4" />
              </div>
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <span className="font-medium">{ch.label}</span>
                  {ch.configured && (
                    <span className="rounded-full bg-success/10 px-2 py-0.5 text-[10px] font-medium text-success">
                      Pre-configured
                    </span>
                  )}
                </div>
                <span className="text-xs text-muted-foreground">
                  {ch.description}
                </span>
              </div>
              <div
                className={cn(
                  "flex size-5 items-center justify-center rounded-full border",
                  isSelected
                    ? "border-primary bg-primary text-primary-foreground"
                    : "border-muted-foreground/30"
                )}
              >
                {isSelected && <Check className="size-3" />}
              </div>
            </button>
          );
        })}
      </div>

      <OnboardingNav
        onNext={handleContinue}
        nextLabel="Continue"
      />
    </div>
  );
}
