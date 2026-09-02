import { useMemo } from "react";
import type { InAppNotification, InAppNotificationType } from "@/lib/types";
import { createRng } from "@/lib/random";

const TYPES: InAppNotificationType[] = [
  "team_add",
  "team_remove",
  "role_change",
  "provider_add",
  "provider_delete",
  "api_key_created",
  "api_key_revoked",
  "project_created",
  "billing_change",
];

const TITLES: Record<InAppNotificationType, string> = {
  team_add: "Team member added",
  team_remove: "Team member removed",
  role_change: "Role changed",
  provider_add: "Provider connected",
  provider_delete: "Provider disconnected",
  api_key_created: "API key created",
  api_key_revoked: "API key revoked",
  project_created: "Project created",
  billing_change: "Plan updated",
  system: "System event",
};

const ACTORS = ["Alice Chen", "Bob Kim", "Carol Wu"];

const REFERENCE_DATE = new Date("2026-08-20T12:00:00.000Z");

function generate(count: number): InAppNotification[] {
  const rng = createRng(7);
  return Array.from({ length: count }, (_, i) => {
    const type = TYPES[Math.floor(rng() * TYPES.length)];
    const createdAt = new Date(REFERENCE_DATE);
    createdAt.setMinutes(createdAt.getMinutes() - Math.floor(rng() * 4320));
    return {
      id: `app_${String(i + 1).padStart(3, "0")}`,
      type,
      title: TITLES[type],
      message: `A new ${type.replace(/_/g, " ")} event was recorded for this project.`,
      actorName: rng() > 0.3 ? ACTORS[Math.floor(rng() * ACTORS.length)] : undefined,
      projectName: rng() > 0.5 ? "Notifie" : "Folded App",
      read: rng() > 0.45,
      createdAt: createdAt.toISOString(),
    };
  });
}

const ALL = generate(36).sort((a, b) => b.createdAt.localeCompare(a.createdAt));

export function useInAppNotifications() {
  return useMemo(() => ALL, []);
}
