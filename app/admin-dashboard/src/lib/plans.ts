// .==================================
// Plan catalog (admin /billing -> Plans tab).
// The Rust backend owns plans (GET/POST/PATCH/DELETE
// /admin/billing/plans); this module validates form input,
// shapes API payloads, formats display strings, and notifies
// listeners after mutations so tabs stay in sync.
// .==================================

import { z } from "zod";
import type { BillingPlan, PlanSubscriber } from "./types";
import { billing } from "./api";

export const PLANS_CHANGED_EVENT = "billing-plans-changed";

export const SUPPORT_TIERS = ["Community", "Email", "Priority", "Dedicated"] as const;

export const CHANNEL_OPTIONS = ["email", "sms", "push", "whatsapp", "webhook", "all"] as const;

// .==================================
// Form input + validation. Nullable numeric fields mean
// "unlimited"; price is dollars unless customPrice is set.
// .==================================

export const PlanInputSchema = z
  .object({
    name: z.string().trim().min(1, "Name is required").max(60, "Keep names under 60 characters"),
    customPrice: z.boolean(),
    priceDollars: z.number().min(0, "Price can't be negative").optional(),
    // Null = monthly only. Percent (0–100, exclusive) or fixed
    // CENTS off the 12-month total.
    yearlyDiscount: z
      .object({
        kind: z.enum(["percent", "fixed"]),
        value: z.number(),
      })
      .nullable(),
    notificationsPerMonth: z.number().int().min(0).nullable(),
    channels: z.array(z.string()).min(1, "Pick at least one channel"),
    teamMembers: z.number().int().min(0).nullable(),
    retentionDays: z.number().int().min(0).nullable(),
    support: z.string().min(1, "Pick a support tier"),
    branding: z.boolean(),
    apiCalls: z.number().int().min(0).nullable(),
  })
  .refine((d) => d.customPrice || (d.priceDollars !== undefined && !Number.isNaN(d.priceDollars)), {
    message: "Enter a price or mark it Custom",
    path: ["priceDollars"],
  })
  .refine((d) => !d.yearlyDiscount || !d.customPrice, {
    message: "Custom-price plans can't offer a yearly discount",
    path: ["yearlyDiscount"],
  })
  .refine(
    (d) => {
      if (!d.yearlyDiscount || d.customPrice) return true;
      if (d.yearlyDiscount.kind === "percent") {
        return d.yearlyDiscount.value > 0 && d.yearlyDiscount.value < 100;
      }
      const base = (d.priceDollars ?? 0) * 100 * 12;
      return d.yearlyDiscount.value > 0 && d.yearlyDiscount.value < base;
    },
    {
      message: "Use 0–100% or a fixed amount below the 12-month total",
      path: ["yearlyDiscount"],
    },
  );

export type PlanInput = z.infer<typeof PlanInputSchema>;

// .==================================
// API payload. Prices in cents (null = custom); fixed
// discounts in cents off the 12-month total.
// .==================================

export type PlanPayload = {
  name: string;
  priceCents: number | null;
  capabilities: {
    notificationsPerMonth: number | null;
    channels: string[];
    teamMembers: number | null;
    retentionDays: number | null;
    support: string;
    branding: boolean;
    apiCalls: number | null;
  };
  yearlyDiscount: { kind: "percent" | "fixed"; value: number } | null;
};

export function toPlanPayload(input: PlanInput): PlanPayload {
  return {
    name: input.name.trim(),
    priceCents: input.customPrice ? null : Math.round((input.priceDollars ?? 0) * 100),
    capabilities: {
      notificationsPerMonth: input.notificationsPerMonth,
      channels: input.channels,
      teamMembers: input.teamMembers,
      retentionDays: input.retentionDays,
      support: input.support,
      branding: input.branding,
      apiCalls: input.apiCalls,
    },
    yearlyDiscount: input.customPrice ? null : input.yearlyDiscount,
  };
}

// .==================================
// Live catalog access. Callers fire notifyPlansChanged()
// after converging local state so every tab re-syncs.
// .==================================

export function notifyPlansChanged() {
  try {
    window.localStorage.setItem(PLANS_PING_KEY, String(Date.now()));
  } catch {
    // storage unavailable — same-tab listeners still fire below
  }
  window.dispatchEvent(new CustomEvent(PLANS_CHANGED_EVENT));
}

const PLANS_PING_KEY = "billing-plans-ping";

export function subscribePlans(listener: () => void) {
  window.addEventListener(PLANS_CHANGED_EVENT, listener);
  const onStorage = (e: StorageEvent) => {
    if (e.key === PLANS_PING_KEY) listener();
  };
  window.addEventListener("storage", onStorage);
  return () => {
    window.removeEventListener(PLANS_CHANGED_EVENT, listener);
    window.removeEventListener("storage", onStorage);
  };
}

// One-time cleanup of the pre-API localStorage mock so stale
// plans can never resurface once the backend owns the catalog.
export function dropLegacyMockStore() {
  try {
    window.localStorage.removeItem("billing-plans-v1");
  } catch {
    // ignore — nothing to clean
  }
}

export async function fetchPlans(): Promise<BillingPlan[]> {
  const { plans } = await billing.listPlans();
  return plans;
}

export async function fetchSubscribers(planId: string): Promise<PlanSubscriber[]> {
  const { subscriptions } = await billing.listSubscribers(planId);
  return subscriptions;
}

export async function savePlan(input: PlanInput, planId?: string): Promise<BillingPlan> {
  const payload = toPlanPayload(input);
  const { plan } = planId
    ? await billing.updatePlan(planId, payload)
    : await billing.createPlan(payload);
  return plan;
}

export type RetireResult = { action: "deleted" | "archived" };

// Delete is blocked server-side while a plan has subscribers;
// callers archive instead (plan stays for existing subscribers
// but can no longer be renewed or sold).
export async function retirePlan(plan: BillingPlan): Promise<RetireResult> {
  if (plan.activeSubscribers > 0) {
    await billing.updatePlan(plan.id, { isActive: false });
    return { action: "archived" };
  }
  await billing.deletePlan(plan.id);
  return { action: "deleted" };
}

// Renewals onto archived (or unknown) plans must be refused.
// Mirrors the backend's plan_retired rule.
export function canRenew(plans: BillingPlan[], planId: string) {
  return plans.some((p) => p.id === planId && p.isActive);
}

// .==================================
// Display helpers shared by the Astro page and islands.
// .==================================

export function planPriceLabel(plan: BillingPlan) {
  if (plan.priceCents === null) return "Custom";
  return `$${(plan.priceCents / 100).toLocaleString()}`;
}

export function planIntervalLabel(plan: BillingPlan) {
  // Plans are monthly by default; yearly = 12 months minus the discount.
  if (plan.priceCents === null) return "tailored";
  return "/mo";
}

function centsToDollars(cents: number) {
  return `$${(cents / 100).toLocaleString(undefined, { maximumFractionDigits: 2 })}`;
}

// 12-month total after the discount. Null for custom-price plans.
export function yearlyPriceCents(plan: BillingPlan): number | null {
  if (plan.priceCents === null) return null;
  const base = plan.priceCents * 12;
  const d = plan.yearlyDiscount;
  if (!d) return base;
  if (d.kind === "percent") return Math.round(base * (1 - d.value / 100));
  return Math.max(0, base - d.value);
}

export function yearlyPriceLabel(plan: BillingPlan) {
  const yearly = yearlyPriceCents(plan);
  if (yearly === null) return null;
  return `${centsToDollars(yearly)}/yr`;
}

// "−20%" or "−$118" — null when there is no discount.
export function yearlySavingsLabel(plan: BillingPlan): string | null {
  if (plan.priceCents === null || !plan.yearlyDiscount) return null;
  const d = plan.yearlyDiscount;
  if (d.kind === "percent") return `−${d.value}%`;
  return `−${centsToDollars(d.value)}`;
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
