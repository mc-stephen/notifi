import { create } from "zustand";
import type { Environment } from "@/lib/types";
import { api, ApiError } from "@/lib/api";
import { useProjectStore } from "./project-store";
import { toast } from "sonner";

type EnvironmentStore = {
  currentEnvironment: Environment;
  pending: boolean;
  /** True after syncFromProject has run at least once. */
  hydrated: boolean;
  /** Call from the topbar after projects hydrate to sync the env from the server. */
  syncFromProject: (env: Environment) => void;
  /** PATCH the server, then commit locally. Reverts on failure. */
  switchEnvironment: (projectId: string, env: Environment) => Promise<void>;
};

export const useEnvironmentStore = create<EnvironmentStore>((set, get) => ({
  currentEnvironment: "development",
  pending: false,
  hydrated: false,

  syncFromProject: (env) => {
    if (!get().pending) set({ currentEnvironment: env, hydrated: true });
  },

  switchEnvironment: async (projectId, env) => {
    if (get().pending || env === get().currentEnvironment) return;

    set({ pending: true });
    try {
      const { project } = await api<{ project: { environment: Environment } }>(
        `/v1/projects/${projectId}/environment`,
        {
          method: "PATCH",
          body: JSON.stringify({ environment: env }),
        },
      );
      // Commit: update both the environment store and the project in the
      // project store so the switcher and badge stay consistent.
      useProjectStore.getState().updateProject({
        ...useProjectStore.getState().currentProject!,
        environment: project.environment,
      });
      set({ currentEnvironment: project.environment });
    } catch (err) {
      const detail =
        err instanceof ApiError
          ? err.message
          : "Check your connection and try again.";
      toast.error("Couldn't switch environment", { description: detail });
      // Revert: environment stays unchanged (never set to `env`).
    } finally {
      set({ pending: false });
    }
  },
}));
