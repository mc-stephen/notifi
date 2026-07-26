import { create } from "zustand";
import type { Organization } from "@/lib/types";

const MOCK_ORGS: Organization[] = [
  {
    id: "org_1",
    name: "Acme Corp",
    slug: "acme",
    plan: "pro",
    createdAt: "2025-01-15T00:00:00Z",
  },
  {
    id: "org_2",
    name: "Startup Labs",
    slug: "startup-labs",
    plan: "starter",
    createdAt: "2025-03-20T00:00:00Z",
  },
];

type OrgStore = {
  organizations: Organization[];
  currentOrg: Organization | null;
  setCurrentOrg: (org: Organization) => void;
};

export const useOrgStore = create<OrgStore>((set) => ({
  organizations: MOCK_ORGS,
  currentOrg: MOCK_ORGS[0],
  setCurrentOrg: (org) => set({ currentOrg: org }),
}));
