import { create } from "zustand";
import type { Project } from "@/lib/types";

const MOCK_PROJECTS: Project[] = [
  {
    id: "proj_1",
    name: "Main App",
    slug: "main-app",
    description: "Primary application notifications",
    createdAt: "2025-01-20T00:00:00Z",
  },
  {
    id: "proj_2",
    name: "Marketing Site",
    slug: "marketing-site",
    description: "Marketing and campaign notifications",
    createdAt: "2025-02-10T00:00:00Z",
  },
  {
    id: "proj_3",
    name: "Mobile App",
    slug: "mobile-app",
    description: "Mobile push notifications",
    createdAt: "2025-04-05T00:00:00Z",
  },
];

type ProjectStore = {
  projects: Project[];
  currentProject: Project | null;
  setCurrentProject: (project: Project) => void;
};

export const useProjectStore = create<ProjectStore>((set) => ({
  projects: MOCK_PROJECTS,
  currentProject: MOCK_PROJECTS[0],
  setCurrentProject: (project) => set({ currentProject: project }),
}));
