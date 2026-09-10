"use client";

import { useMemo, useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import {
  CreditCard,
  CheckCircle2,
  Zap,
  Bell,
  ArrowUpRight,
  TriangleAlert,
} from "lucide-react";
import { useBilling } from "@/hooks";
import { useProjectStore } from "@/store/project-store";
import type { BillingCycle, BillingPlan } from "@/lib/types";

function dollars(cents: number) {
  const value = cents / 100;
  return `$${value.toLocaleString(undefined, { maximumFractionDigits: value % 1 === 0 ? 0 : 2 })}`;
}

function limitLabel(value: number | null, unit: string) {
  if (value === null) return `Unlimited ${unit}`;
  return `${value.toLocaleString()} ${unit}`;
}

function capabilityRows(plan: BillingPlan) {
  const c = plan.capabilities;
  return [
    limitLabel(c.notificationsPerMonth, "notifications/mo"),
    c.channels.includes("all") ? "All channels" : `${c.channels.join(", ")} channels`,
    limitLabel(c.teamMembers, "team members"),
    c.retentionDays === null ? "Unlimited retention" : `${c.retentionDays}-day retention`,
    `${c.support} support`,
    c.branding ? "Custom branding" : "No custom branding",
    c.apiCalls === null ? "Unlimited API calls" : `${c.apiCalls.toLocaleString()} API calls/mo`,
  ];
}

function planPrice(plan: BillingPlan, cycle: BillingCycle) {
  if (plan.priceCents === null) return { main: "Custom", sub: "tailored", savings: null as string | null };
  if (cycle === "monthly" || plan.yearlyPriceCents === null) {
    return { main: dollars(plan.priceCents), sub: "/mo", savings: null as string | null };
  }
  const savings =
    plan.yearlyDiscount?.kind === "percent"
      ? `−${plan.yearlyDiscount.value}%`
      : plan.yearlyDiscount
        ? `−${dollars(plan.yearlyDiscount.value)}`
        : null;
  return { main: dollars(plan.yearlyPriceCents), sub: "/yr", savings };
}

export default function BillingPage() {
  const project = useProjectStore((s) => s.currentProject);
  const { plans, subscription, loading, error, refresh, subscribe, cancel } = useBilling();
  const [cycle, setCycle] = useState<BillingCycle>("monthly");
  const [acting, setActing] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [confirmCancel, setConfirmCancel] = useState(false);

  const currentPlan = subscription?.plan ?? null;

  const usagePct = useMemo(() => {
    if (!subscription) return 0;
    const { notificationsUsed, notificationsLimit } = subscription.usage;
    if (notificationsLimit === null || notificationsLimit <= 0) return 0;
    return Math.min(100, Math.round((notificationsUsed / notificationsLimit) * 100));
  }, [subscription]);

  async function handleSubscribe(plan: BillingPlan) {
    setActing(plan.id);
    setActionError(null);
    try {
      await subscribe(plan.id, cycle);
    } catch (e) {
      setActionError(e instanceof Error ? e.message : "Could not change plan");
    } finally {
      setActing(null);
    }
  }

  async function handleCancel() {
    setActing("cancel");
    setActionError(null);
    try {
      await cancel();
      setConfirmCancel(false);
    } catch (e) {
      setActionError(e instanceof Error ? e.message : "Could not cancel subscription");
    } finally {
      setActing(null);
    }
  }

  async function handleKeep() {
    if (!currentPlan) return;
    setActing("keep");
    setActionError(null);
    try {
      await subscribe(currentPlan.id, subscription?.billingCycle ?? "monthly");
    } catch (e) {
      setActionError(e instanceof Error ? e.message : "Could not keep subscription");
    } finally {
      setActing(null);
    }
  }

  return (
    <div className="space-y-6">
      <PageHeader
        title="Billing"
        description={project ? `Subscription and payment for ${project.name}` : "Subscription and payment"}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Billing" }]}
        actions={
          <Button size="sm" variant="outline" className="gap-1.5" disabled title="Online payments land with the billing provider">
            <CreditCard className="size-3.5" /> Update payment
          </Button>
        }
      />

      {actionError && (
        <Card className="border-destructive/30 bg-destructive/5">
          <CardContent className="pt-5 text-sm text-destructive flex items-center gap-2">
            <TriangleAlert className="size-4 shrink-0" />
            {actionError}
          </CardContent>
        </Card>
      )}

      {loading ? (
        <div className="space-y-4" aria-label="Loading billing">
          {/* Current plan skeleton */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <Skeleton className="h-5 w-28" />
                <Skeleton className="h-5 w-16" />
              </div>
            </CardHeader>
            <CardContent>
              <div className="grid gap-6 md:grid-cols-2">
                <div className="space-y-4">
                  <Skeleton className="h-9 w-40" />
                  <Skeleton className="h-4 w-56" />
                  <div className="flex gap-2">
                    <Skeleton className="h-7 w-28" />
                    <Skeleton className="h-7 w-36" />
                  </div>
                </div>
                <div className="space-y-3">
                  <Skeleton className="h-4 w-24" />
                  <Skeleton className="h-4 w-full" />
                  <Skeleton className="h-4 w-5/6" />
                  <Skeleton className="h-4 w-4/6" />
                </div>
              </div>
            </CardContent>
          </Card>
          {/* Usage skeleton */}
          <Card>
            <CardContent className="pt-5 space-y-3">
              <Skeleton className="h-4 w-44" />
              <Skeleton className="h-8 w-24" />
              <Skeleton className="h-2 w-full" />
            </CardContent>
          </Card>
          {/* Plans grid skeleton */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between gap-4">
                <Skeleton className="h-5 w-36" />
                <Skeleton className="h-8 w-44" />
              </div>
            </CardHeader>
            <CardContent>
              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                {[0, 1, 2, 3].map((i) => (
                  <div key={i} className="rounded-lg border p-4 space-y-3">
                    <Skeleton className="h-5 w-20" />
                    <Skeleton className="h-8 w-24" />
                    <Separator />
                    <Skeleton className="h-3 w-full" />
                    <Skeleton className="h-3 w-5/6" />
                    <Skeleton className="h-3 w-4/6" />
                    <Skeleton className="h-7 w-full" />
                  </div>
                ))}
              </div>
            </CardContent>
          </Card>
        </div>
      ) : error ? (
        <Card>
          <CardContent className="pt-5 space-y-3">
            <p className="text-sm font-medium">Couldn&apos;t load billing</p>
            <p className="text-sm text-muted-foreground">{error}</p>
            <Button size="sm" variant="outline" onClick={refresh}>
              Try again
            </Button>
          </CardContent>
        </Card>
      ) : !project || !subscription || !currentPlan ? (
        <Card>
          <CardContent className="pt-5 space-y-3">
            <p className="text-sm font-medium">No project selected</p>
            <p className="text-sm text-muted-foreground">
              Create or select a project to see its subscription. New projects start on the Free plan.
            </p>
          </CardContent>
        </Card>
      ) : (
        <>
          {/* Current plan */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle>Current Plan</CardTitle>
                <div className="flex items-center gap-2">
                  {subscription.status !== "active" && (
                    <Badge variant="outline" className="capitalize border-warning/30 text-warning">
                      {subscription.status.replace("_", " ")}
                    </Badge>
                  )}
                  <Badge className="gap-1 bg-primary text-primary-foreground">
                    <Zap className="size-3" /> {currentPlan.name}
                  </Badge>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="grid gap-6 md:grid-cols-2">
                <div className="space-y-4">
                  {(() => {
                    const price = planPrice(currentPlan, subscription.billingCycle);
                    return (
                      <div className="text-3xl font-bold">
                        {price.main}
                        <span className="text-lg font-normal text-muted-foreground">{price.sub}</span>
                        {price.savings && (
                          <span className="ml-2 align-middle text-xs font-medium text-success bg-success/10 border border-success/20 rounded-md px-1.5 py-0.5">
                            {price.savings} yearly
                          </span>
                        )}
                      </div>
                    );
                  })()}
                  <p className="text-sm text-muted-foreground">
                    {(() => {
                      const renews = new Date(subscription.periodEnd).toLocaleDateString();
                      if (subscription.status === "cancelled") {
                        return "Your subscription is cancelled. Resubscribe below to keep sending.";
                      }
                      if (subscription.cancelAtPeriodEnd) {
                        return `Cancels on ${renews} — then reverts to the Free plan.`;
                      }
                      return `Your plan renews on ${renews}.`;
                    })()}
                  </p>
                <div className="flex gap-2">
                  {subscription.status === "cancelled" ? (
                    <Button size="sm" onClick={() => document.getElementById("plans-grid")?.scrollIntoView({ behavior: "smooth" })}>
                      Resubscribe
                    </Button>
                  ) : subscription.cancelAtPeriodEnd ? (
                    <Button size="sm" disabled={acting === "keep"} onClick={handleKeep}>
                      {acting === "keep" ? "Keeping…" : "Keep my plan"}
                    </Button>
                  ) : currentPlan.id === "free" ? (
                    <>
                      <Button size="sm" onClick={() => document.getElementById("plans-grid")?.scrollIntoView({ behavior: "smooth" })}>
                        Upgrade plan
                      </Button>
                      <span className="text-xs text-muted-foreground self-center">
                        Free is the default plan and can&apos;t be cancelled.
                      </span>
                    </>
                  ) : confirmCancel ? (
                    <>
                      <Button
                        size="sm"
                        variant="destructive"
                        disabled={acting === "cancel"}
                        onClick={handleCancel}
                      >
                        {acting === "cancel" ? "Cancelling…" : "Confirm cancel"}
                      </Button>
                      <Button size="sm" variant="outline" onClick={() => setConfirmCancel(false)}>
                        Keep plan
                      </Button>
                    </>
                  ) : (
                    <>
                      <Button size="sm" onClick={() => document.getElementById("plans-grid")?.scrollIntoView({ behavior: "smooth" })}>
                        Upgrade plan
                      </Button>
                      <Button size="sm" variant="outline" onClick={() => setConfirmCancel(true)}>
                        Cancel subscription
                      </Button>
                    </>
                  )}
                </div>
                {confirmCancel && !subscription.cancelAtPeriodEnd && currentPlan.id !== "free" && (
                  <p className="text-xs text-muted-foreground">
                    Keeps {currentPlan.name} until {new Date(subscription.periodEnd).toLocaleDateString()}, then Free.
                  </p>
                )}
                </div>
                <div className="space-y-3">
                  <h4 className="text-sm font-medium">Plan includes</h4>
                  {capabilityRows(currentPlan).map((feature) => (
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
          <Card>
            <CardContent className="pt-5">
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Notifications this period</span>
                <Bell className="size-4 text-muted-foreground" />
              </div>
              <div className="text-2xl font-bold mt-1">
                {subscription.usage.notificationsUsed.toLocaleString()}
              </div>
              <p className="text-xs text-muted-foreground mt-1">
                {subscription.usage.notificationsLimit === null
                  ? "Unlimited"
                  : `of ${subscription.usage.notificationsLimit.toLocaleString()} (${usagePct}%)`}
              </p>
              {subscription.usage.notificationsLimit !== null && (
                <div className="h-2 bg-muted rounded-full overflow-hidden mt-3">
                  <div className="h-full bg-primary rounded-full" style={{ width: `${usagePct}%` }} />
                </div>
              )}
            </CardContent>
          </Card>

          {/* Plans */}
          <Card id="plans-grid">
            <CardHeader>
              <div className="flex items-center justify-between gap-4">
                <CardTitle>Available Plans</CardTitle>
                <div className="flex items-center gap-1 rounded-lg border border-border bg-card p-1" role="group" aria-label="Billing period">
                  {(["monthly", "yearly"] as BillingCycle[]).map((c) => (
                    <button
                      key={c}
                      type="button"
                      onClick={() => setCycle(c)}
                      aria-pressed={cycle === c}
                      className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors capitalize ${
                        cycle === c
                          ? "bg-primary text-primary-foreground"
                          : "text-muted-foreground hover:text-foreground hover:bg-muted"
                      }`}
                    >
                      {c === "monthly" ? "Monthly" : "Yearly"}
                    </button>
                  ))}
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                {plans.map((plan) => {
                  const isCurrent = plan.id === currentPlan.id && subscription.status === "active";
                  const price = planPrice(plan, cycle);
                  const isCustom = plan.priceCents === null;
                  return (
                    <div
                      key={plan.id}
                      className={`rounded-lg border p-4 space-y-3 ${
                        isCurrent ? "border-primary bg-primary/5" : ""
                      }`}
                    >
                      <div className="flex items-center justify-between">
                        <h4 className="font-medium">{plan.name}</h4>
                        {isCurrent && <Badge className="text-[10px]">Current</Badge>}
                      </div>
                      <div className="text-2xl font-bold">
                        {price.main}
                        <span className="text-sm font-normal text-muted-foreground">{price.sub}</span>
                        {price.savings && (
                          <span className="ml-1.5 align-middle text-[11px] font-medium text-success">
                            {price.savings}
                          </span>
                        )}
                      </div>
                      <Separator />
                      <ul className="space-y-2">
                        {capabilityRows(plan).map((f, i) => (
                          <li key={`${f}-${i}`} className="flex items-center gap-2 text-xs">
                            <CheckCircle2 className="size-3 text-success shrink-0" />
                            <span>{f}</span>
                          </li>
                        ))}
                      </ul>
                      {!isCurrent &&
                        (isCustom ? (
                          <Button variant="outline" size="sm" className="w-full" render={<a href="/support" />}>
                            Contact sales
                            <ArrowUpRight className="size-3 ml-1" />
                          </Button>
                        ) : (
                          <Button
                            variant="outline"
                            size="sm"
                            className="w-full"
                            disabled={acting === plan.id}
                            onClick={() => handleSubscribe(plan)}
                          >
                            {acting === plan.id ? "Switching…" : "Upgrade"}
                            <ArrowUpRight className="size-3 ml-1" />
                          </Button>
                        ))}
                    </div>
                  );
                })}
              </div>
              {plans.length === 0 && (
                <p className="text-sm text-muted-foreground">No plans available right now.</p>
              )}
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
