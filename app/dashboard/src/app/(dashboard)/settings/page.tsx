"use client";

import Link from "next/link";
import { cn } from "@/lib/utils";
import { PageHeader } from "@/components/dashboard/shared/page-header";
import { Key, Webhook, CreditCard, Users } from "lucide-react";

const settingsCards = [
  {
    title: "API Keys",
    description: "Manage authentication credentials for the API.",
    href: "/settings/api-keys",
    icon: Key,
  },
  {
    title: "Webhooks",
    description: "Configure event webhook endpoints.",
    href: "/settings/webhooks",
    icon: Webhook,
  },
  {
    title: "Billing",
    description: "View usage and manage your subscription.",
    href: "#",
    icon: CreditCard,
  },
  {
    title: "Team",
    description: "Manage team members and permissions.",
    href: "#",
    icon: Users,
  },
];

const gradientMap: Record<string, string> = {
  Key: "from-blue-500/40 to-blue-500",
  Webhook: "from-purple-500/40 to-purple-500",
  CreditCard: "from-emerald-500/40 to-emerald-500",
  Users: "from-amber-500/40 to-amber-500",
};

export default function SettingsPage() {
  return (
    <div className="mx-auto w-full max-w-7xl space-y-6 p-5">
      <PageHeader
        title="Settings"
        description="Billing, team permissions, and workspace configuration."
      />

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        {settingsCards.map((card) => {
          const Icon = card.icon;
          const Component = card.href.startsWith("/") ? Link : "a";
          const props = card.href.startsWith("/")
            ? { href: card.href }
            : { href: card.href, className: "cursor-pointer" };

          return (
            <Component key={card.title} {...props}>
              <div className="card-lift overflow-hidden rounded-xl border border-border/40 bg-card">
                <div className={cn("h-1 w-full bg-gradient-to-r", gradientMap[card.title])} />
                <div className="flex items-center gap-3 p-5">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
                    <Icon className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <p className="text-sm font-medium text-foreground">{card.title}</p>
                    <p className="text-xs text-muted-foreground mt-0.5">{card.description}</p>
                  </div>
                </div>
              </div>
            </Component>
          );
        })}
      </div>
    </div>
  );
}
