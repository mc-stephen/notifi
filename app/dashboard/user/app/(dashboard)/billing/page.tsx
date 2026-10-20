"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ArrowUpRight, CircleHelp, Loader2 } from "lucide-react";
import { useBilling } from "@/hooks";
import { useProjectStore } from "@/store/project-store";
import type { BillingCycle, BillingPlan } from "@/lib/types";
import { UsageBar } from "@/components/custom/billing/usage-bar";
import { PlanAlert } from "@/components/custom/billing/plan-alert";
import { PlanTile } from "@/components/custom/billing/plan-tile";
import { PaymentMethodSection } from "@/components/custom/billing/payment-method-section";
import { InvoiceHistory } from "@/components/custom/billing/invoice-history";
import { planPrice } from "@/components/custom/billing/billing-display";

export default function BillingPage() {
  const router = useRouter();
  const project = useProjectStore((s) => s.currentProject);
  const {
    plans,
    subscription,
    receipts,
    loading,
    error,
    refresh,
    subscribe,
    cancel,
  } = useBilling();
  const [cycle, setCycle] = useState<BillingCycle>("monthly");
  const [acting, setActing] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [confirmCancel, setConfirmCancel] = useState(false);
  const [plansOpen, setPlansOpen] = useState(false);

  const currentPlan = subscription?.plan ?? null;

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
      setActionError(
        e instanceof Error ? e.message : "Could not cancel subscription",
      );
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
      setActionError(
        e instanceof Error ? e.message : "Could not keep subscription",
      );
    } finally {
      setActing(null);
    }
  }
  return (
    <div className="space-y-6">
      <PageHeader
        title="Billing"
        description={
          project
            ? `Subscription and payment for ${project.name}`
            : "Subscription and payment"
        }
        actions={
          <div className="flex items-center gap-2">
            <span className="hidden text-xs text-muted-foreground sm:inline">
              Online payments land with the billing provider
            </span>
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Billing help"
              title="Billing help"
              onClick={() => router.push("/support")}
            >
              <CircleHelp className="size-4" />
            </Button>
          </div>
        }
      />

      {actionError && (
        <PlanAlert
          tone="destructive"
          title="Billing action failed"
          message={actionError}
        />
      )}
      {loading ? (
        <Card>
          <CardContent>
            <div
              className="flex items-center justify-center py-16 text-muted-foreground"
              aria-label="Loading billing"
            >
              <Loader2 className="size-5 animate-spin mr-2" /> Loading billing…
            </div>
          </CardContent>
        </Card>
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
              Create or select a project to see its subscription. New projects
              start on the Free plan.
            </p>
          </CardContent>
        </Card>
      ) : (
        <>
          <div className="grid gap-4 lg:grid-cols-5">
            {/* Current plan */}
            <Card className="lg:col-span-3">
              <CardContent className="pt-5 space-y-5">
                <div className="flex items-start justify-between gap-4">
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="font-semibold">
                        Current Plan ({currentPlan.name})
                      </h3>
                      <Badge
                        variant="secondary"
                        className="text-[10px] font-medium capitalize"
                      >
                        {subscription.billingCycle === "yearly"
                          ? "Yearly"
                          : "Monthly"}
                      </Badge>
                      {subscription.status !== "active" && (
                        <Badge
                          variant="outline"
                          className="capitalize border-warning/30 text-warning text-[10px] font-medium"
                        >
                          {subscription.status.replace("_", " ")}
                        </Badge>
                      )}
                    </div>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {(() => {
                        const renews = new Date(
                          subscription.periodEnd,
                        ).toLocaleDateString();
                        if (subscription.status === "cancelled") {
                          return "Your subscription is cancelled. Resubscribe below to keep sending.";
                        }
                        if (subscription.cancelAtPeriodEnd) {
                          return `Cancels on ${renews} — then reverts to the Free plan.`;
                        }
                        return `Active until ${renews}.`;
                      })()}
                    </p>
                  </div>
                  {(() => {
                    const price = planPrice(
                      currentPlan,
                      subscription.billingCycle,
                    );
                    return (
                      <div className="shrink-0 text-right">
                        <span className="text-5xl font-bold tracking-tight">
                          {price.main}
                        </span>
                        <span className="text-lg font-normal text-muted-foreground">
                          {price.sub}
                        </span>
                        {price.savings && (
                          <span className="ml-2 align-middle text-xs font-medium text-success bg-success/10 border border-success/20 rounded-md px-1.5 py-0.5">
                            {price.savings} yearly
                          </span>
                        )}
                      </div>
                    );
                  })()}
                </div>

                <UsageBar
                  label="Notifications used this period"
                  used={subscription.usage.notificationsUsed}
                  limit={subscription.usage.notificationsLimit}
                />

                {subscription.status === "cancelled" ? (
                  <PlanAlert
                    tone="destructive"
                    title="Subscription cancelled"
                    message={`Keeps ${currentPlan.name} until ${new Date(subscription.periodEnd).toLocaleDateString()}, then Free.`}
                    action={
                      <Button size="sm" onClick={() => setPlansOpen(true)}>
                        Resubscribe
                      </Button>
                    }
                  />
                ) : subscription.cancelAtPeriodEnd ? (
                  <PlanAlert
                    tone="warning"
                    title="Cancelling at period end"
                    message={`Keeps ${currentPlan.name} until ${new Date(subscription.periodEnd).toLocaleDateString()}, then Free.`}
                    action={
                      <Button
                        size="sm"
                        disabled={acting === "keep"}
                        onClick={handleKeep}
                      >
                        {acting === "keep" ? "Keeping…" : "Keep my plan"}
                      </Button>
                    }
                  />
                ) : subscription.status !== "active" ? (
                  <PlanAlert
                    tone="warning"
                    title={`Subscription ${subscription.status.replace("_", " ")}`}
                    message="Resolve billing to keep sending without interruption."
                  />
                ) : null}

                <div className="flex flex-wrap items-center gap-2">
                  {currentPlan.id === "free" ? (
                    <>
                      <Button size="sm" onClick={() => setPlansOpen(true)}>
                        Upgrade plan
                      </Button>
                      <span className="text-xs text-muted-foreground">
                        Free is the default plan and can&apos;t be cancelled.
                      </span>
                    </>
                  ) : !subscription.cancelAtPeriodEnd && confirmCancel ? (
                    <>
                      <Button
                        size="sm"
                        variant="destructive"
                        disabled={acting === "cancel"}
                        onClick={handleCancel}
                      >
                        {acting === "cancel" ? "Cancelling…" : "Confirm cancel"}
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => setConfirmCancel(false)}
                      >
                        Keep plan
                      </Button>
                    </>
                  ) : !subscription.cancelAtPeriodEnd ? (
                    <>
                      <Button size="sm" onClick={() => setPlansOpen(true)}>
                        Upgrade plan
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => setConfirmCancel(true)}
                      >
                        Cancel subscription
                      </Button>
                    </>
                  ) : null}
                </div>
                {confirmCancel &&
                  !subscription.cancelAtPeriodEnd &&
                  currentPlan.id !== "free" && (
                    <p className="text-xs text-muted-foreground">
                      Keeps {currentPlan.name} until{" "}
                      {new Date(subscription.periodEnd).toLocaleDateString()},
                      then Free.
                    </p>
                  )}

                <div className="border-t border-border pt-3">
                  <Button
                    variant="link"
                    size="sm"
                    className="h-auto gap-1 p-0 text-sm"
                    onClick={() => setPlansOpen(true)}
                  >
                    See all plans <ArrowUpRight className="size-3.5" />
                  </Button>
                </div>
              </CardContent>
            </Card>

            {/* Payment method */}
            <Card className="lg:col-span-2">
              <CardContent className="pt-5">
                <PaymentMethodSection methods={[]} />
              </CardContent>
            </Card>
          </div>

          {/* Billing history */}
          <InvoiceHistory receipts={receipts} projectName={project.name} />

          {/* Plans popup */}
          <Dialog open={plansOpen} onOpenChange={setPlansOpen}>
            <DialogContent className="flex max-h-[85vh] flex-col overflow-hidden sm:max-w-4xl">
              <DialogHeader>
                {/* pr-10 reserves the DialogContent close button, which floats
                    absolute top-2 right-2 over this row otherwise. */}
                <div className="flex items-center justify-between gap-4 pr-10">
                  <DialogTitle>Available Plans</DialogTitle>
                  <div
                    className="flex items-center gap-1 rounded-lg border border-border bg-card p-1"
                    role="group"
                    aria-label="Billing period"
                  >
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
              </DialogHeader>
              <div className="min-h-0 flex-1 overflow-y-auto pr-1">
                <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                  {plans.map((plan) => (
                    <PlanTile
                      key={plan.id}
                      plan={plan}
                      cycle={cycle}
                      isCurrent={
                        plan.id === currentPlan.id &&
                        subscription.status === "active"
                      }
                      busy={acting === plan.id}
                      onSubscribe={handleSubscribe}
                    />
                  ))}
                </div>
                {plans.length === 0 && (
                  <p className="text-sm text-muted-foreground">
                    No plans available right now.
                  </p>
                )}
              </div>
            </DialogContent>
          </Dialog>
        </>
      )}
    </div>
  );
}
