"use client";

import { create } from "zustand";
import type { Environment } from "@/lib/types";

interface EnvironmentState {
  environment: Environment;
  setEnvironment: (env: Environment) => void;
  toggleEnvironment: () => void;
}

export const useEnvironmentStore = create<EnvironmentState>((set) => ({
  environment: "development",
  setEnvironment: (env) => set({ environment: env }),
  toggleEnvironment: () =>
    set((state) => ({
      environment: state.environment === "production" ? "development" : "production",
    })),
}));
