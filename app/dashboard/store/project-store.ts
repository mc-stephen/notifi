import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { Project } from "@/lib/types";
import { api } from "@/lib/api";

type ProjectStore = {
  projects: Project[];
  currentProject: Project | null;
  currentProjectId: string | null;
  loading: boolean;
  loadProjects: () => Promise<void>;
  setCurrentProject: (project: Project) => void;
  updateProject: (project: Project) => void;
  createProject: (name: string, description?: string) => Promise<Project>;
  _hasHydrated: boolean;
  _setHasHydrated: (v: boolean) => void;
};

export const useProjectStore = create<ProjectStore>()(
  persist(
    (set, get) => ({
      projects: [],
      currentProject: null,
      currentProjectId: null,
      loading: false,
      _hasHydrated: false,
      _setHasHydrated: (v) => set({ _hasHydrated: v }),

      loadProjects: async () => {
        if (get().loading) return;
        set({ loading: true });
        try {
          const { projects } = await api<{ projects: Project[] }>("/v1/projects");
          const current = get().currentProject;
          const savedId = get().currentProjectId;
          let resolved: Project | null = null;
          if (current && projects.some((p) => p.id === current.id)) {
            resolved = current;
          } else if (savedId) {
            resolved = projects.find((p) => p.id === savedId) ?? null;
          }
          if (!resolved) resolved = projects[0] ?? null;
          set({ projects, currentProject: resolved });
        } catch {
          // Unauthenticated or network error — leave state empty; the auth
          // layout gate will redirect to login before the user sees this.
        } finally {
          set({ loading: false });
        }
      },

      setCurrentProject: (project) =>
        set({ currentProject: project, currentProjectId: project.id }),

      updateProject: (updated) =>
        set((state) => ({
          projects: state.projects.map((p) =>
            p.id === updated.id ? updated : p,
          ),
          currentProject:
            state.currentProject?.id === updated.id ? updated : state.currentProject,
        })),

      createProject: async (name, description) => {
        const { project } = await api<{ project: Project }>(
          "/v1/projects",
          {
            method: "POST",
            body: JSON.stringify({ name, description: description || null }),
          },
        );
        set((state) => ({
          projects: [...state.projects, project],
          currentProject: project,
          currentProjectId: project.id,
        }));
        return project;
      },
    }),
    {
      name: "notifi-project",
      partialize: (state) => ({ currentProjectId: state.currentProjectId }),
      skipHydration: true,
      onRehydrateStorage: () => (state) => {
        state?._setHasHydrated(true);
      },
    },
  ),
);
