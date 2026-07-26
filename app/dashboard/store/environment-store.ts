import { create } from "zustand";
import type { Environment } from "@/lib/types";

type EnvironmentStore = {
  currentEnvironment: Environment;
  setEnvironment: (env: Environment) => void;
};

export const useEnvironmentStore = create<EnvironmentStore>((set) => ({
  currentEnvironment: "development",
  setEnvironment: (env) => set({ currentEnvironment: env }),
}));
