import { create } from "zustand";
import type { Project } from "@/lib/types";
import { api } from "@/lib/api";

type ProjectStore = {
  projects: Project[];
  currentProject: Project | null;
  loading: boolean;
  loadProjects: () => Promise<void>;
  setCurrentProject: (project: Project) => void;
  updateProject: (project: Project) => void;
};

export const useProjectStore = create<ProjectStore>((set, get) => ({
  projects: [],
  currentProject: null,
  loading: false,

  loadProjects: async () => {
    if (get().loading) return;
    set({ loading: true });
    try {
      const { projects } = await api<{ projects: Project[] }>("/v1/projects");
      const current = get().currentProject;
      set({
        projects,
        currentProject:
          current && projects.some((p) => p.id === current.id)
            ? current
            : projects[0] ?? null,
      });
    } catch {
      // Unauthenticated or network error — leave state empty; the auth
      // layout gate will redirect to login before the user sees this.
    } finally {
      set({ loading: false });
    }
  },

  setCurrentProject: (project) => set({ currentProject: project }),

  updateProject: (updated) =>
    set((state) => ({
      projects: state.projects.map((p) =>
        p.id === updated.id ? updated : p,
      ),
      currentProject:
        state.currentProject?.id === updated.id ? updated : state.currentProject,
    })),
}));
