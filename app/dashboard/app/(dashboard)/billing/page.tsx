"use client";

import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  CreditCard,
  CheckCircle2,
  Zap,
  Users,
  Bell,
  ArrowUpRight,
} from "lucide-react";

export default function BillingPage() {
  return (
    <div className="space-y-6">
      <PageHeader
        title="Billing"
        description="Subscription and payment"
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Billing" }]}
        actions={
          <Button size="sm" className="gap-1.5">
            <CreditCard className="size-3.5" /> Update payment
          </Button>
        }
      />

      {/* Current plan */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Current Plan</CardTitle>
            <Badge className="gap-1 bg-primary text-primary-foreground">
              <Zap className="size-3" /> Pro
            </Badge>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid gap-6 md:grid-cols-2">
            <div className="space-y-4">
              <div className="text-3xl font-bold">$49<span className="text-lg font-normal text-muted-foreground">/month</span></div>
              <p className="text-sm text-muted-foreground">Your plan renews on July 15, 2025.</p>
              <div className="flex gap-2">
                <Button size="sm">Upgrade plan</Button>
                <Button size="sm" variant="outline">Cancel subscription</Button>
              </div>
            </div>
            <div className="space-y-3">
              <h4 className="text-sm font-medium">Plan includes</h4>
              {[
                "100,000 notifications/month",
                "All channels (Email, SMS, Push, Webhook)",
                "5 team members",
                "30-day retention",
                "Priority support",
                "Custom branding",
              ].map((feature) => (
                <div key={feature} className="flex items-center gap-2 text-sm">
                  <CheckCircle2 className="size-3.5 text-success" />
                  <span>{feature}</span>
                </div>
              ))}
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Usage */}
      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Notifications</span>
              <Bell className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">67,234</div>
            <p className="text-xs text-muted-foreground mt-1">of 100,000 (67%)</p>
            <div className="h-2 bg-muted rounded-full overflow-hidden mt-3">
              <div className="h-full bg-primary rounded-full" style={{ width: "67%" }} />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Team Members</span>
              <Users className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">6</div>
            <p className="text-xs text-muted-foreground mt-1">of 5 (over limit)</p>
            <div className="h-2 bg-muted rounded-full overflow-hidden mt-3">
              <div className="h-full bg-warning rounded-full" style={{ width: "100%" }} />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">API Calls</span>
              <Zap className="size-4 text-muted-foreground" />
            </div>
            <div className="text-2xl font-bold mt-1">1.2M</div>
            <p className="text-xs text-success mt-1">Unlimited</p>
            <div className="h-2 bg-muted rounded-full overflow-hidden mt-3">
              <div className="h-full bg-success rounded-full" style={{ width: "40%" }} />
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Plans comparison */}
      <Card>
        <CardHeader>
          <CardTitle>Available Plans</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid gap-4 md:grid-cols-4">
            {[
              { name: "Free", price: "$0", features: ["1,000 notifications/mo", "1 channel", "1 team member"] },
              { name: "Starter", price: "$19", features: ["10,000 notifications/mo", "3 channels", "3 team members"] },
              { name: "Pro", price: "$49", features: ["100,000 notifications/mo", "All channels", "5 team members"], current: true },
              { name: "Enterprise", price: "Custom", features: ["Unlimited notifications", "All channels", "Unlimited team members", "SLA", "Dedicated support"] },
            ].map((plan) => (
              <div
                key={plan.name}
                className={`rounded-lg border p-4 space-y-3 ${
                  plan.current ? "border-primary bg-primary/5" : ""
                }`}
              >
                <div className="flex items-center justify-between">
                  <h4 className="font-medium">{plan.name}</h4>
                  {plan.current && <Badge className="text-[10px]">Current</Badge>}
                </div>
                <div className="text-2xl font-bold">{plan.price}<span className="text-sm font-normal text-muted-foreground">/mo</span></div>
                <Separator />
                <ul className="space-y-2">
                  {plan.features.map((f, i) => (
                    <li key={`${f}-${i}`} className="flex items-center gap-2 text-xs">
                      <CheckCircle2 className="size-3 text-success shrink-0" />
                      <span>{f}</span>
                    </li>
                  ))}
                </ul>
                {!plan.current && (
                  <Button variant="outline" size="sm" className="w-full">
                    {plan.price === "Custom" ? "Contact sales" : "Upgrade"}
                    <ArrowUpRight className="size-3 ml-1" />
                  </Button>
                )}
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
