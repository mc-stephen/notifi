// Mock users for the notifications compose modal (user/org pickers).
// TODO: replace with the live /v1/admin/users endpoint when the
// notifications page goes live (orgs have no backend yet).

export type MockUser = {
  id: string;
  name: string;
  email: string;
  status: "active" | "suspended";
  organization?: string;
};

export const mockUsers: MockUser[] = [
  { id: "usr_01", name: "Sarah Chen", email: "sarah.chen@techcorp.io", status: "active", organization: "TechCorp" },
  { id: "usr_02", name: "Marcus Johnson", email: "marcus@startupxyz.com", status: "active", organization: "StartupXYZ" },
  { id: "usr_03", name: "Priya Patel", email: "priya@finserve.com", status: "active", organization: "FinServe" },
  { id: "usr_04", name: "Alex Kim", email: "alex.kim@digitalagency.co", status: "active", organization: "Digital Agency" },
  { id: "usr_05", name: "Jordan Lee", email: "jordan@games.io", status: "suspended", organization: "Games.io" },
  { id: "usr_06", name: "Taylor Swift", email: "taylor@musicapp.io", status: "active", organization: "MusicApp" },
  { id: "usr_07", name: "Sam Rivera", email: "sam@healthplus.org", status: "active", organization: "HealthPlus" },
  { id: "usr_08", name: "Morgan Davis", email: "morgan@healthtech.org", status: "active", organization: "HealthTech" },
  { id: "usr_09", name: "Riley Wilson", email: "riley@fineducation.edu", status: "active", organization: "FineEducation" },
  { id: "usr_10", name: "Quinn Martinez", email: "quinn@logisticsplus.com", status: "active", organization: "LogisticsPlus" },
];
