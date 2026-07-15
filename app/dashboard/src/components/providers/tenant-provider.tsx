"use client";

import { createContext, useContext, type ReactNode } from "react";
import { useTenantStore } from "@/store/tenant-store";
import { useEnvironmentStore } from "@/store/environment-store";
import type { Tenant, Environment } from "@/lib/types";

interface TenantContextValue {
  tenant: Tenant;
  environment: Environment;
}

const TenantContext = createContext<TenantContextValue>({
  tenant: { id: "", name: "", slug: "" },
  environment: "development",
});

export function useTenantContext() {
  return useContext(TenantContext);
}

export function TenantProvider({ children }: { children: ReactNode }) {
  const currentTenant = useTenantStore((s) => s.currentTenant);
  const environment = useEnvironmentStore((s) => s.environment);

  return (
    <TenantContext.Provider value={{ tenant: currentTenant, environment }}>
      {children}
    </TenantContext.Provider>
  );
}
