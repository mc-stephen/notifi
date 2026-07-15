"use client";

import { useTenantStore } from "@/store/tenant-store";
import { Building2, ChevronDown } from "lucide-react";

export function OrgSelector() {
  const { tenants, currentTenant, setTenant } = useTenantStore();

  return (
    <div className="relative">
      <select
        value={currentTenant.id}
        onChange={(e) => {
          const t = tenants.find((t) => t.id === e.target.value);
          if (t) setTenant(t);
        }}
        className="flex h-8 items-center gap-2 rounded-lg border border-border bg-transparent pl-8 pr-7 text-sm font-medium appearance-none cursor-pointer transition-colors hover:bg-secondary focus:outline-none focus:ring-2 focus:ring-ring"
      >
        {tenants.map((t) => (
          <option key={t.id} value={t.id}>
            {t.name}
          </option>
        ))}
      </select>
      <Building2 className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
      <ChevronDown className="pointer-events-none absolute right-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
    </div>
  );
}
