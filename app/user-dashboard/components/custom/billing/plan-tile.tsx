import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { CheckCircle2, ArrowUpRight } from "lucide-react";
import type { BillingCycle, BillingPlan } from "@/lib/types";
import { capabilityRows, planPrice } from "./billing-display";

type PlanTileProps = {
  plan: BillingPlan;
  cycle: BillingCycle;
  isCurrent: boolean;
  busy: boolean;
  onSubscribe: (plan: BillingPlan) => void;
};

/**
 * One pricing-grid tile: name + current badge, big price + savings,
 * capability list, and the subscribe/contact action.
 */
export function PlanTile({ plan, cycle, isCurrent, busy, onSubscribe }: PlanTileProps) {
  const price = planPrice(plan, cycle);
  const isCustom = plan.priceCents === null;

  return (
    <div
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
            disabled={busy}
            onClick={() => onSubscribe(plan)}
          >
            {busy ? "Switching…" : "Upgrade"}
            <ArrowUpRight className="size-3 ml-1" />
          </Button>
        ))}
    </div>
  );
}
