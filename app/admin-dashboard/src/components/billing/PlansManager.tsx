// .==================================
// Plans editor (admin /billing -> Plans tab).
// Add / edit / remove plans; removal of a plan that still
// has subscribers archives it instead (no renewals onto it).
// Live catalog via GET/POST/PATCH/DELETE /admin/billing/plans.
// .==================================

import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import { z } from "zod";
import type { BillingPlan, PlanSubscriber } from "@/lib/types";
import { ApiError } from "@/lib/api";
import {
  CHANNEL_OPTIONS,
  SUPPORT_TIERS,
  PlanInputSchema,
  capabilityRows,
  dropLegacyMockStore,
  fetchPlans,
  fetchSubscribers,
  notifyPlansChanged,
  planIntervalLabel,
  planPriceLabel,
  retirePlan,
  savePlan,
  subscribePlans,
  yearlyPriceCents,
  yearlyPriceLabel,
  yearlySavingsLabel,
  type PlanInput,
} from "@/lib/plans";

// .==================================
// Form state. Numeric fields stay strings until submit so
// empty inputs can be distinguished from zero.
// .==================================

type FormState = {
  name: string;
  customPrice: boolean;
  priceDollars: string;
  // Yearly offer: 12 months for less. `discountValue` is % or $ per kind.
  offerYearly: boolean;
  discountKind: "percent" | "fixed";
  discountValue: string;
  notifications: string;
  unlimitedNotifications: boolean;
  channels: string[];
  teamMembers: string;
  unlimitedTeam: boolean;
  retention: string;
  unlimitedRetention: boolean;
  support: string;
  branding: boolean;
  apiCalls: string;
  unlimitedApi: boolean;
};

function emptyForm(): FormState {
  return {
    name: "",
    customPrice: false,
    priceDollars: "",
    offerYearly: false,
    discountKind: "percent",
    discountValue: "20",
    notifications: "10000",
    unlimitedNotifications: false,
    channels: ["email"],
    teamMembers: "3",
    unlimitedTeam: false,
    retention: "30",
    unlimitedRetention: false,
    support: "Email",
    branding: false,
    apiCalls: "",
    unlimitedApi: true,
  };
}

function formFromPlan(plan: BillingPlan): FormState {
  const c = plan.capabilities;
  const d = plan.yearlyDiscount;
  return {
    name: plan.name,
    customPrice: plan.priceCents === null,
    priceDollars: plan.priceCents === null ? "" : String(plan.priceCents / 100),
    offerYearly: d !== null,
    discountKind: d?.kind ?? "percent",
    discountValue:
      d === null ? "20" : d.kind === "percent" ? String(d.value) : String(d.value / 100),
    notifications: c.notificationsPerMonth === null ? "" : String(c.notificationsPerMonth),
    unlimitedNotifications: c.notificationsPerMonth === null,
    channels: [...c.channels],
    teamMembers: c.teamMembers === null ? "" : String(c.teamMembers),
    unlimitedTeam: c.teamMembers === null,
    retention: c.retentionDays === null ? "" : String(c.retentionDays),
    unlimitedRetention: c.retentionDays === null,
    support: c.support,
    branding: c.branding,
    apiCalls: c.apiCalls === null ? "" : String(c.apiCalls),
    unlimitedApi: c.apiCalls === null,
  };
}

function toOptionalInt(raw: string, unlimited: boolean): number | null | undefined {
  if (unlimited) return null;
  const trimmed = raw.trim();
  if (!/^\d+$/.test(trimmed)) return undefined;
  return parseInt(trimmed, 10);
}

function toYearlyDiscount(form: FormState): PlanInput["yearlyDiscount"] {
  if (!form.offerYearly || form.customPrice) return null;
  const trimmed = form.discountValue.trim();
  // NaN deliberately fails zod's number check with a mapped message below.
  const num = /^\d+(\.\d+)?$/.test(trimmed) ? Number(trimmed) : NaN;
  return {
    kind: form.discountKind,
    value: form.discountKind === "percent" ? num : Math.round(num * 100),
  };
}

function toInput(form: FormState): PlanInput {
  const priceTrimmed = form.priceDollars.trim();
  return PlanInputSchema.parse({
    name: form.name,
    customPrice: form.customPrice,
    priceDollars:
      form.customPrice || priceTrimmed === "" ? undefined : Number(priceTrimmed),
    yearlyDiscount: toYearlyDiscount(form),
    notificationsPerMonth: toOptionalInt(form.notifications, form.unlimitedNotifications),
    channels: form.channels,
    teamMembers: toOptionalInt(form.teamMembers, form.unlimitedTeam),
    retentionDays: toOptionalInt(form.retention, form.unlimitedRetention),
    support: form.support,
    branding: form.branding,
    apiCalls: toOptionalInt(form.apiCalls, form.unlimitedApi),
  });
}

const FIELD_LABELS: Record<string, string> = {
  name: "Name",
  priceDollars: "Price",
  yearlyDiscount: "Yearly discount",
  channels: "Channels",
  notificationsPerMonth: "Notifications",
  teamMembers: "Team members",
  retentionDays: "Retention",
  support: "Support",
  apiCalls: "API calls",
};

function friendlyErrors(err: z.ZodError): Record<string, string> {
  const out: Record<string, string> = {};
  for (const issue of err.issues) {
    const key = String(issue.path[0] ?? "form");
    if (!out[key]) {
      out[key] =
        key === "yearlyDiscount"
          ? "Enter a valid discount (0–100% or a fixed amount below the 12-month total)"
          : issue.code === "invalid_type"
            ? `Enter a whole number for ${(FIELD_LABELS[key] ?? key).toLowerCase()} or tick Unlimited`
            : issue.message;
    }
  }
  return out;
}

const inputCls =
  "flex h-9 w-full rounded-lg border border-input bg-card px-3 py-1.5 text-sm transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-ring/50 disabled:opacity-50";

export default function PlansManager() {
  // No seed content: skeletons paint until the live catalog arrives,
  // so mock numbers are never shown as fact.
  const [plans, setPlans] = useState<BillingPlan[]>([]);
  const [modal, setModal] = useState<{ mode: "add" | "edit"; planId?: string } | null>(null);
  const [form, setForm] = useState<FormState>(emptyForm);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<BillingPlan | null>(null);
  const [menuId, setMenuId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [retiring, setRetiring] = useState(false);
  const [loadingPlans, setLoadingPlans] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [subsByPlan, setSubsByPlan] = useState<Record<string, PlanSubscriber[]>>({});
  const [subsLoadingId, setSubsLoadingId] = useState<string | null>(null);
  const [subsErrorId, setSubsErrorId] = useState<string | null>(null);
  const nameRef = useRef<HTMLInputElement>(null);

  function toLoadError(err: unknown) {
    if (err instanceof ApiError && err.status === 401) {
      return "Your session has expired. Please sign out and sign in again.";
    }
    return "Could not load plans. Check that the API is running and try again.";
  }

  // Live catalog: skeletons paint first, then the backend refreshes.
  const refresh = useCallback(async () => {
    setLoadingPlans(true);
    setLoadError(null);
    try {
      setPlans(await fetchPlans());
      setSubsByPlan({});
      setExpandedId(null);
    } catch (err) {
      setLoadError(toLoadError(err));
    } finally {
      setLoadingPlans(false);
    }
  }, []);

  // Who holds this plan — lazy per-card drill-down.
  async function toggleSubscribers(plan: BillingPlan) {
    if (expandedId === plan.id) {
      setExpandedId(null);
      return;
    }
    setExpandedId(plan.id);
    setSubsErrorId(null);
    if (subsByPlan[plan.id]) return;
    setSubsLoadingId(plan.id);
    try {
      const subs = await fetchSubscribers(plan.id);
      setSubsByPlan((prev) => ({ ...prev, [plan.id]: subs }));
    } catch {
      setSubsErrorId(plan.id);
    } finally {
      setSubsLoadingId(null);
    }
  }

  useEffect(() => {
    dropLegacyMockStore();
    refresh();
    // Other tabs (or another browser tab) mutated the catalog.
    return subscribePlans(() => {
      fetchPlans().then(setPlans).catch(() => {});
    });
  }, [refresh]);

  useEffect(() => {
    if (modal) {
      setFieldErrors({});
      setFormError(null);
      nameRef.current?.focus();
    }
  }, [modal]);

  useEffect(() => {
    if (!notice) return;
    const t = setTimeout(() => setNotice(null), 3500);
    return () => clearTimeout(t);
  }, [notice]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setModal(null);
        setConfirm(null);
        setMenuId(null);
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  function openAdd() {
    setForm(emptyForm());
    setModal({ mode: "add" });
  }

  function openEdit(plan: BillingPlan) {
    setForm(formFromPlan(plan));
    setMenuId(null);
    setModal({ mode: "edit", planId: plan.id });
  }

  function set<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((f) => ({ ...f, [key]: value }));
  }

  function toggleChannel(ch: string) {
    setForm((f) => {
      if (ch === "all") {
        return { ...f, channels: f.channels.includes("all") ? [] : ["all"] };
      }
      const rest = f.channels.filter((c) => c !== "all");
      const next = rest.includes(ch) ? rest.filter((c) => c !== ch) : [...rest, ch];
      return { ...f, channels: next };
    });
  }

  async function handleSubmit(e: { preventDefault: () => void }) {
    e.preventDefault();
    setFieldErrors({});
    setFormError(null);
    let input: PlanInput;
    try {
      input = toInput(form);
    } catch (err) {
      if (err instanceof z.ZodError) {
        setFieldErrors(friendlyErrors(err));
        return;
      }
      setFormError("Could not save this plan. Please check the fields.");
      return;
    }
    setSaving(true);
    try {
      const saved = await savePlan(input, modal?.mode === "edit" ? modal.planId : undefined);
      setPlans(await fetchPlans());
      notifyPlansChanged();
      setNotice(
        modal?.mode === "edit" ? `Plan "${saved.name}" updated.` : `Plan "${saved.name}" created.`,
      );
      setModal(null);
    } catch (err) {
      setFormError(
        err instanceof ApiError && err.status === 409
          ? "A plan with this name already exists."
          : err instanceof Error
            ? err.message
            : "Could not save this plan.",
      );
    } finally {
      setSaving(false);
    }
  }

  async function handleRetire() {
    if (!confirm || retiring) return;
    setRetiring(true);
    try {
      const result = await retirePlan(confirm);
      setPlans(await fetchPlans());
      notifyPlansChanged();
      setNotice(
        result.action === "archived"
          ? `"${confirm.name}" archived — ${confirm.activeSubscribers} subscriber${confirm.activeSubscribers === 1 ? " keeps" : "s keep"} it until expiry, no renewals onto it.`
          : `Plan "${confirm.name}" removed.`,
      );
    } catch (err) {
      if (err instanceof ApiError && err.status === 409) {
        // Gained subscribers between render and confirm — refresh counts
        // and let the admin archive instead.
        fetchPlans().then(setPlans).catch(() => {});
        setNotice("That plan now has subscribers — archive it instead of removing it.");
      } else {
        setNotice(err instanceof Error ? err.message : "Could not remove this plan.");
      }
    } finally {
      setRetiring(false);
      setConfirm(null);
      setMenuId(null);
    }
  }

  const sorted = [...plans].sort((a, b) => Number(b.isActive) - Number(a.isActive));
  const editingName = modal?.mode === "edit" ? form.name.trim() || "plan" : "";

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-4">
        <p className="text-sm text-muted-foreground">
          {loadingPlans && plans.length === 0 ? (
            "Loading plans…"
          ) : (
            <>
              {plans.filter((p) => p.isActive).length} active plan{plans.length === 1 ? "" : "s"}
              {plans.some((p) => !p.isActive) && (
                <span> · {plans.filter((p) => !p.isActive).length} archived</span>
              )}
            </>
          )}
        </p>
        <button
          type="button"
          onClick={openAdd}
          className="inline-flex items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-sm transition-colors hover:bg-primary/80 shrink-0"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M5 12h14" /><path d="M12 5v14" /></svg>
          Add plan
        </button>
      </div>

      {loadError && (
        <div className="rounded-xl border bg-card px-4 py-12 text-center space-y-3">
          <p className="text-sm text-destructive">{loadError}</p>
          <div className="flex items-center justify-center gap-2">
            <button
              type="button"
              onClick={refresh}
              className="inline-flex items-center justify-center rounded-lg border border-input bg-background px-4 py-2 text-sm font-medium hover:bg-muted transition-colors"
            >
              Try again
            </button>
            <a
              href="/login"
              className="inline-flex items-center justify-center rounded-lg border border-input bg-background px-4 py-2 text-sm font-medium hover:bg-muted transition-colors"
            >
              Sign in again
            </a>
          </div>
        </div>
      )}

      {notice && (
        <div className="rounded-lg border border-success/20 bg-success/5 p-3 text-sm text-success">
          {notice}
        </div>
      )}

      {loadingPlans && plans.length === 0 && !loadError ? (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4" aria-label="Loading plans">
          {[0, 1, 2, 3].map((i) => (
            <div key={i} className="rounded-xl border bg-card p-5 space-y-4 animate-pulse">
              <div className="h-5 w-2/3 rounded bg-muted" />
              <div className="h-8 w-1/2 rounded bg-muted" />
              <div className="border-t border-border" />
              <div className="space-y-2">
                <div className="h-3 w-full rounded bg-muted" />
                <div className="h-3 w-5/6 rounded bg-muted" />
                <div className="h-3 w-4/6 rounded bg-muted" />
              </div>
            </div>
          ))}
        </div>
      ) : (
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {sorted.map((plan) => (
          <div
            key={plan.id}
            className={`rounded-xl border bg-card p-5 space-y-4 relative ${plan.isActive ? "" : "opacity-75"}`}
          >
            <div className="flex items-center justify-between gap-2">
              <h3 className="font-semibold text-foreground truncate">{plan.name}</h3>
              <div className="flex items-center gap-1 shrink-0">
                {!plan.isActive ? (
                  <span className="inline-flex items-center rounded-md border border-border bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
                    Archived
                  </span>
                ) : plan.activeSubscribers > 0 ? (
                  <button
                    type="button"
                    onClick={() => toggleSubscribers(plan)}
                    aria-expanded={expandedId === plan.id}
                    title="Show subscribers"
                    className="inline-flex items-center gap-1 rounded-md bg-secondary px-2 py-0.5 text-xs font-medium text-secondary-foreground hover:bg-secondary/70 transition-colors"
                  >
                    {plan.activeSubscribers} active
                    <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className={`transition-transform ${expandedId === plan.id ? "rotate-180" : ""}`}><path d="m6 9 6 6 6-6" /></svg>
                  </button>
                ) : (
                  <span className="inline-flex items-center rounded-md bg-secondary px-2 py-0.5 text-xs font-medium text-secondary-foreground">
                    0 active
                  </span>
                )}
                <button
                  type="button"
                  aria-label={`Actions for ${plan.name}`}
                  onClick={() => setMenuId((id) => (id === plan.id ? null : plan.id))}
                  className="p-1.5 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="1" /><circle cx="12" cy="5" r="1" /><circle cx="12" cy="19" r="1" /></svg>
                </button>
              </div>
            </div>
            {menuId === plan.id && (
              <>
                <button
                  type="button"
                  aria-label="Close menu"
                  onClick={() => setMenuId(null)}
                  className="fixed inset-0 z-10 cursor-default bg-transparent"
                />
                <div className="absolute right-3 top-12 z-20 w-36 rounded-lg border border-border bg-card shadow-lg py-1">
                  <button
                    type="button"
                    onClick={() => openEdit(plan)}
                    className="flex w-full items-center px-3 py-2 text-xs font-medium text-foreground hover:bg-muted transition-colors text-left"
                  >
                    Edit
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setMenuId(null);
                      setConfirm(plan);
                    }}
                    className="flex w-full items-center px-3 py-2 text-xs font-medium text-destructive hover:bg-destructive/5 transition-colors text-left"
                  >
                    {plan.activeSubscribers > 0 ? "Archive" : "Remove"}
                  </button>
                </div>
              </>
            )}
            <div>
              <div className="text-2xl font-bold text-foreground">
                {planPriceLabel(plan)}
                <span className="text-sm font-normal text-muted-foreground">{planIntervalLabel(plan)}</span>
              </div>
              {yearlyPriceLabel(plan) && (
                <p className="text-xs text-muted-foreground mt-1">
                  {yearlyPriceLabel(plan)}
                  {yearlySavingsLabel(plan) && (
                    <span className="ml-1.5 inline-flex items-center rounded-md bg-success/10 text-success border border-success/20 px-1.5 py-px text-[11px] font-medium">
                      {yearlySavingsLabel(plan)}
                    </span>
                  )}
                </p>
              )}
            </div>
            <div className="border-t border-border" />
            <ul className="space-y-2">
              {capabilityRows(plan).map((row) => (
                <li key={row} className="flex items-center gap-2 text-xs text-foreground">
                  <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-success shrink-0"><polyline points="20 6 9 17 4 12" /></svg>
                  <span>{row}</span>
                </li>
              ))}
            </ul>
            {!plan.isActive && (
              <p className="text-xs text-muted-foreground">
                {plan.activeSubscribers} subscriber{plan.activeSubscribers === 1 ? "" : "s"} remain{plan.activeSubscribers === 1 ? "s" : ""} until expiry — renewals onto this plan are blocked.
              </p>
            )}
            {expandedId === plan.id && (
              <div className="border-t border-border pt-3 space-y-2">
                <p className="text-xs font-medium text-muted-foreground">Subscribers</p>
                {subsLoadingId === plan.id ? (
                  <div className="space-y-2 animate-pulse">
                    <div className="h-8 rounded bg-muted" />
                    <div className="h-8 rounded bg-muted" />
                  </div>
                ) : subsErrorId === plan.id ? (
                  <p className="text-xs text-destructive">
                    Could not load subscribers. If this keeps happening, restart the API server — this view needs the plan-subscriptions endpoint.
                  </p>
                ) : (subsByPlan[plan.id] ?? []).length === 0 ? (
                  <p className="text-xs text-muted-foreground">No active subscribers.</p>
                ) : (
                  <ul className="space-y-2">
                    {(subsByPlan[plan.id] ?? []).map((sub) => (
                      <li key={sub.subscriptionId} className="rounded-lg border border-border/60 px-3 py-2">
                        {/* Billing is per project: the project leads, the
                            owner is just the contact. */}
                        <p className="text-xs font-medium text-foreground truncate">
                          {sub.projectName}
                        </p>
                        <p className="text-[11px] text-muted-foreground mt-0.5 capitalize">
                          {sub.billingCycle} · {sub.status.replace("_", " ")} · renews {new Date(sub.periodEnd).toLocaleDateString()}
                        </p>
                        <p className="text-[11px] text-muted-foreground truncate">
                          {sub.customerName ?? "Unknown owner"}{sub.customerEmail ? ` · ${sub.customerEmail}` : ""}
                        </p>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}
          </div>
        ))}
      </div>
      )}

      {!loadingPlans && !loadError && sorted.length === 0 && (
        <div className="rounded-xl border bg-card px-4 py-12 text-center">
          <p className="text-sm font-medium text-foreground">No plans yet</p>
          <p className="text-xs text-muted-foreground mt-1">Add your first plan to start selling subscriptions.</p>
        </div>
      )}

      {modal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center" role="dialog" aria-modal="true">
          <div className="absolute inset-0 bg-black/50" onClick={() => setModal(null)} />
          <div className="relative z-10 w-full max-w-lg mx-4 rounded-xl border bg-card shadow-xl max-h-[90vh] overflow-y-auto">
            <div className="flex items-center justify-between p-5 border-b border-border">
              <div>
                <h2 className="font-semibold text-foreground">
                  {modal.mode === "add" ? "Add plan" : `Edit ${editingName}`}
                </h2>
                <p className="text-sm text-muted-foreground mt-0.5">
                  {modal.mode === "add"
                    ? "Set the price and what the plan entails."
                    : "Changes apply to new subscriptions immediately."}
                </p>
              </div>
              <button
                type="button"
                aria-label="Close dialog"
                onClick={() => setModal(null)}
                className="p-1.5 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors shrink-0"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M18 6 6 18" /><path d="m6 6 12 12" /></svg>
              </button>
            </div>
            <form onSubmit={handleSubmit} className="p-5 space-y-4">
              <div className="space-y-2">
                <label htmlFor="plan-name" className="text-sm font-medium text-foreground">Plan name</label>
                <input
                  id="plan-name"
                  ref={nameRef}
                  type="text"
                  value={form.name}
                  onChange={(e) => set("name", e.target.value)}
                  placeholder="e.g. Growth"
                  className={inputCls}
                />
                {fieldErrors.name && <p className="text-xs text-destructive">{fieldErrors.name}</p>}
              </div>

              <div className="space-y-2">
                <label htmlFor="plan-price" className="text-sm font-medium text-foreground">Price (USD / month)</label>
                <input
                  id="plan-price"
                  type="number"
                  min="0"
                  step="0.01"
                  value={form.priceDollars}
                  disabled={form.customPrice}
                  onChange={(e) => set("priceDollars", e.target.value)}
                  placeholder="49"
                  className={inputCls}
                />
                {fieldErrors.priceDollars && <p className="text-xs text-destructive">{fieldErrors.priceDollars}</p>}
              </div>
              <label className="flex items-center gap-2 text-sm text-foreground">
                <input
                  type="checkbox"
                  checked={form.customPrice}
                  onChange={(e) => set("customPrice", e.target.checked)}
                  className="size-4 rounded border-input accent-primary"
                />
                Custom price (enterprise-style, tailored quote)
              </label>

              <YearlyDiscountField
                form={form}
                onChange={setForm}
                error={fieldErrors.yearlyDiscount}
              />

              <div className="border-t border-border" />
              <p className="text-sm font-medium text-foreground">What the plan entails</p>

              <LimitField
                id="plan-notifications"
                label="Notifications / month"
                value={form.notifications}
                unlimited={form.unlimitedNotifications}
                onValue={(v) => set("notifications", v)}
                onUnlimited={(v) => set("unlimitedNotifications", v)}
                error={fieldErrors.notificationsPerMonth}
              />

              <div className="space-y-2">
                <span className="text-sm font-medium text-foreground">Channels</span>
                <div className="flex flex-wrap gap-2">
                  {CHANNEL_OPTIONS.map((ch) => (
                    <label
                      key={ch}
                      className={`inline-flex items-center gap-1.5 rounded-md border px-2.5 py-1.5 text-xs font-medium cursor-pointer transition-colors ${form.channels.includes(ch) ? "border-primary/40 bg-primary/5 text-foreground" : "border-input text-muted-foreground hover:text-foreground"}`}
                    >
                      <input
                        type="checkbox"
                        checked={form.channels.includes(ch)}
                        onChange={() => toggleChannel(ch)}
                        className="size-3.5 accent-primary"
                      />
                      <span className="capitalize">{ch === "all" ? "All channels" : ch}</span>
                    </label>
                  ))}
                </div>
                {fieldErrors.channels && <p className="text-xs text-destructive">{fieldErrors.channels}</p>}
              </div>

              <LimitField
                id="plan-team"
                label="Team members"
                value={form.teamMembers}
                unlimited={form.unlimitedTeam}
                onValue={(v) => set("teamMembers", v)}
                onUnlimited={(v) => set("unlimitedTeam", v)}
                error={fieldErrors.teamMembers}
              />

              <LimitField
                id="plan-retention"
                label="Retention (days)"
                value={form.retention}
                unlimited={form.unlimitedRetention}
                onValue={(v) => set("retention", v)}
                onUnlimited={(v) => set("unlimitedRetention", v)}
                error={fieldErrors.retentionDays}
              />

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <label htmlFor="plan-support" className="text-sm font-medium text-foreground">Support</label>
                  <select
                    id="plan-support"
                    value={form.support}
                    onChange={(e) => set("support", e.target.value)}
                    className={inputCls}
                  >
                    {SUPPORT_TIERS.map((t) => (
                      <option key={t} value={t}>{t}</option>
                    ))}
                  </select>
                  {fieldErrors.support && <p className="text-xs text-destructive">{fieldErrors.support}</p>}
                </div>
                <div className="space-y-2">
                  <span className="text-sm font-medium text-foreground">Branding</span>
                  <label className="flex h-9 items-center gap-2 text-sm text-foreground">
                    <input
                      type="checkbox"
                      checked={form.branding}
                      onChange={(e) => set("branding", e.target.checked)}
                      className="size-4 rounded border-input accent-primary"
                    />
                    Custom branding
                  </label>
                </div>
              </div>

              <LimitField
                id="plan-api"
                label="API calls / month"
                value={form.apiCalls}
                unlimited={form.unlimitedApi}
                onValue={(v) => set("apiCalls", v)}
                onUnlimited={(v) => set("unlimitedApi", v)}
                error={fieldErrors.apiCalls}
              />

              {formError && (
                <div className="rounded-lg border border-destructive/20 bg-destructive/5 p-3 text-sm text-destructive">
                  {formError}
                </div>
              )}

              <div className="flex justify-end gap-2 pt-1">
                <button
                  type="button"
                  onClick={() => setModal(null)}
                  className="inline-flex items-center justify-center gap-2 rounded-lg border border-input bg-background px-4 py-2 text-sm font-medium hover:bg-muted transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={saving}
                  className="inline-flex items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-sm transition-colors hover:bg-primary/80 disabled:pointer-events-none disabled:opacity-50"
                >
                  {modal.mode === "add" ? "Create plan" : "Save changes"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {confirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center" role="dialog" aria-modal="true">
          <div className="absolute inset-0 bg-black/50" onClick={() => setConfirm(null)} />
          <div className="relative z-10 w-full max-w-sm mx-4 rounded-xl border bg-card shadow-xl">
            <div className="p-5 space-y-2">
              <h2 className="font-semibold text-foreground">
                {confirm.activeSubscribers > 0 ? `Archive ${confirm.name}?` : `Remove ${confirm.name}?`}
              </h2>
              <p className="text-sm text-muted-foreground">
                {confirm.activeSubscribers > 0
                  ? `${confirm.activeSubscribers} subscriber${confirm.activeSubscribers === 1 ? " keeps" : "s keep"} this plan until expiry, but nobody can renew onto it or buy it anymore.`
                  : "This plan has no subscribers. Removing it cannot be undone from here."}
              </p>
            </div>
            <div className="flex justify-end gap-2 p-5 pt-0">
              <button
                type="button"
                onClick={() => setConfirm(null)}
                className="inline-flex items-center justify-center gap-2 rounded-lg border border-input bg-background px-4 py-2 text-sm font-medium hover:bg-muted transition-colors"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleRetire}
                disabled={retiring}
                className="inline-flex items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-sm transition-colors hover:bg-primary/80 disabled:pointer-events-none disabled:opacity-50"
              >
                {retiring
                  ? "Working…"
                  : confirm.activeSubscribers > 0
                    ? "Archive plan"
                    : "Remove plan"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// .==================================
// Yearly offer: percent or fixed amount off the 12-month
// total, with a live preview of what users would pay.
// .==================================

function YearlyDiscountField({
  form,
  onChange,
  error,
}: {
  form: FormState;
  onChange: Dispatch<SetStateAction<FormState>>;
  error?: string;
}) {
  const disabled = form.customPrice;
  const update = (patch: Partial<FormState>) => onChange((f) => ({ ...f, ...patch }));

  const monthlyCents =
    !disabled && /^\d+(\.\d+)?$/.test(form.priceDollars.trim())
      ? Math.round(Number(form.priceDollars.trim()) * 100)
      : null;
  const discountValid = /^\d+(\.\d+)?$/.test(form.discountValue.trim());
  const previewPlan =
    form.offerYearly && monthlyCents !== null && discountValid
      ? ({
          priceCents: monthlyCents,
          yearlyDiscount: {
            kind: form.discountKind,
            value:
              form.discountKind === "percent"
                ? Number(form.discountValue.trim())
                : Math.round(Number(form.discountValue.trim()) * 100),
          },
        } as BillingPlan)
      : null;
  const previewYearly = previewPlan ? yearlyPriceCents(previewPlan) : null;
  const previewBase = monthlyCents !== null ? monthlyCents * 12 : null;

  return (
    <div className="space-y-3 rounded-lg border border-border/70 p-3">
      <label className={`flex items-center gap-2 text-sm font-medium text-foreground ${disabled ? "opacity-50" : ""}`}>
        <input
          type="checkbox"
          checked={form.offerYearly && !disabled}
          disabled={disabled}
          onChange={(e) => update({ offerYearly: e.target.checked })}
          className="size-4 rounded border-input accent-primary"
        />
        Offer yearly billing (12 months for less)
      </label>
      {disabled && (
        <p className="text-xs text-muted-foreground">Custom-price plans can't offer a yearly discount.</p>
      )}
      {form.offerYearly && !disabled && (
        <div className="space-y-3">
          <div className="flex gap-2">
            {(
              [
                { kind: "percent", label: "% Percent off" },
                { kind: "fixed", label: "$ Fixed amount off" },
              ] as const
            ).map((opt) => (
              <button
                key={opt.kind}
                type="button"
                onClick={() => update({ discountKind: opt.kind })}
                className={`flex-1 rounded-lg border px-3 py-1.5 text-xs font-medium transition-colors ${form.discountKind === opt.kind ? "border-primary/40 bg-primary/5 text-foreground" : "border-input text-muted-foreground hover:text-foreground"}`}
              >
                {opt.label}
              </button>
            ))}
          </div>
          <div className="space-y-2">
            <label htmlFor="plan-discount" className="text-sm font-medium text-foreground">
              {form.discountKind === "percent" ? "Discount (%)" : "Discount (USD off the yearly total)"}
            </label>
            <input
              id="plan-discount"
              type="number"
              min="0"
              step={form.discountKind === "percent" ? "1" : "0.01"}
              value={form.discountValue}
              onChange={(e) => update({ discountValue: e.target.value })}
              placeholder={form.discountKind === "percent" ? "20" : "100"}
              className={inputCls}
            />
          </div>
          {previewYearly !== null && previewBase !== null && (
            <p className="text-xs text-muted-foreground">
              Yearly:{" "}
              <span className="font-medium text-foreground">
                ${(previewYearly / 100).toLocaleString(undefined, { maximumFractionDigits: 2 })}/yr
              </span>
              {previewBase - previewYearly > 0 && (
                <span className="text-success">
                  {" "}· save ${((previewBase - previewYearly) / 100).toLocaleString(undefined, { maximumFractionDigits: 2 })} vs monthly
                </span>
              )}
            </p>
          )}
        </div>
      )}
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}

// .==================================
// Number + Unlimited row shared by the four capped fields.
// .==================================

function LimitField({
  id,
  label,
  value,
  unlimited,
  onValue,
  onUnlimited,
  error,
}: {
  id: string;
  label: string;
  value: string;
  unlimited: boolean;
  onValue: (v: string) => void;
  onUnlimited: (v: boolean) => void;
  error?: string;
}) {
  return (
    <div className="space-y-2">
      <label htmlFor={id} className="text-sm font-medium text-foreground">{label}</label>
      <div className="flex items-center gap-3">
        <input
          id={id}
          type="number"
          min="0"
          step="1"
          value={value}
          disabled={unlimited}
          onChange={(e) => onValue(e.target.value)}
          placeholder="Unlimited"
          className={inputCls}
        />
        <label className="flex items-center gap-1.5 text-xs text-muted-foreground whitespace-nowrap cursor-pointer">
          <input
            type="checkbox"
            checked={unlimited}
            onChange={(e) => onUnlimited(e.target.checked)}
            className="size-4 rounded border-input accent-primary"
          />
          Unlimited
        </label>
      </div>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
