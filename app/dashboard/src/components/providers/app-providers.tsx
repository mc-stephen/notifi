"use client";

import type { ReactNode } from "react";
import { ThemeProvider } from "./theme-provider";
import { TenantProvider } from "./tenant-provider";

export function AppProviders({ children }: { children: ReactNode }) {
  return (
    <ThemeProvider>
      <TenantProvider>{children}</TenantProvider>
    </ThemeProvider>
  );
}
