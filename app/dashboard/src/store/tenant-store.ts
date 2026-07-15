"use client";

import { create } from "zustand";
import type { Tenant } from "@/lib/types";
import { MOCK_TENANTS } from "@/lib/constants";

interface TenantState {
  tenants: Tenant[];
  currentTenant: Tenant;
  setTenant: (tenant: Tenant) => void;
}

export const useTenantStore = create<TenantState>((set) => ({
  tenants: MOCK_TENANTS,
  currentTenant: MOCK_TENANTS[0],
  setTenant: (tenant) => set({ currentTenant: tenant }),
}));
