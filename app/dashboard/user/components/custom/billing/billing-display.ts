import type { BillingCycle, BillingPlan } from "@/lib/types";

export function dollars(cents: number) {
  const value = cents / 100;
  return `$${value.toLocaleString(undefined, { maximumFractionDigits: value % 1 === 0 ? 0 : 2 })}`;
}

export function limitLabel(value: number | null, unit: string) {
  if (value === null) return `Unlimited ${unit}`;
  return `${value.toLocaleString()} ${unit}`;
}

export function capabilityRows(plan: BillingPlan) {
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

export function planPrice(plan: BillingPlan, cycle: BillingCycle) {
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

/** Whole percent of a capped usage, clamped 0–100. Null cap → 0. */
export function usagePercent(used: number, limit: number | null) {
  if (limit === null || limit <= 0) return 0;
  return Math.min(100, Math.round((used / limit) * 100));
}
