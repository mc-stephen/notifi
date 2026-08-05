/**
 * Status roll-up — the single source of truth for every badge on the site.
 * Pure functions over the domain types (src/lib/types.ts); mirrors the
 * algorithm the Rust backend will reproduce in M6.
 *
 * Priority per component (design §9.1):
 *   1. open incident severity  → major_outage | partial_outage | degraded
 *   2. in-progress maintenance → maintenance
 *   3. probe state
 *   4. operational
 */
import {
  SEVERITY_RANK,
  SEVERITY_TO_STATE,
  STATE_RANK,
  type Component,
  type ComponentState,
  type Incident,
  type Maintenance,
  type Subsystem,
} from "./types";

export function isIncidentOpen(incident: Incident): boolean {
  return incident.status !== "resolved";
}

/** Incidents still open at time `now`, newest first. */
export function openIncidents(incidents: Incident[], now: Date = new Date()): Incident[] {
  return incidents.filter((i) => isIncidentOpen(i)).sort((a, b) => b.startedAt.localeCompare(a.startedAt));
}

export function resolvedIncidents(incidents: Incident[], now: Date = new Date()): Incident[] {
  return incidents.filter((i) => i.resolvedAt && !isIncidentOpen(i)).sort((a, b) => b.resolvedAt!.localeCompare(a.resolvedAt!));
}

/** Highest open severity affecting a component, or undefined. */
export function activeIncidentFor(
  component: Component,
  incidents: Incident[],
  now: Date = new Date(),
): Incident | undefined {
  let worst: Incident | undefined;
  for (const incident of incidents) {
    if (!isIncidentOpen(incident)) continue;
    if (!incident.affectedComponents.includes(component.slug)) continue;
    if (!worst || SEVERITY_RANK[incident.severity] > SEVERITY_RANK[worst.severity]) worst = incident;
  }
  return worst;
}

/** In-progress maintenance affecting a component, or undefined. */
export function activeMaintenanceFor(
  component: Component,
  maintenance: Maintenance[],
  now: Date = new Date(),
): Maintenance | undefined {
  return maintenance.find((m) => m.status === "in_progress" && m.affectedComponents.includes(component.slug));
}

/** Scheduled (future) maintenance affecting a component. */
export function scheduledMaintenanceFor(
  component: Component,
  maintenance: Maintenance[],
  now: Date = new Date(),
): Maintenance | undefined {
  return maintenance.find((m) => m.status === "scheduled" && m.affectedComponents.includes(component.slug));
}

export interface ComponentHealth extends Component {
  state: ComponentState;
  /** Open incident driving the current state, if any. */
  incident?: Incident;
  /** In-progress maintenance driving the state, if any. */
  maintenance?: Maintenance;
  /** Future maintenance — does not change state, only flags the banner. */
  maintenanceScheduled?: boolean;
}

/** Resolve one component's state with full explainability. */
export function resolveComponentHealth(
  component: Component,
  incidents: Incident[],
  maintenance: Maintenance[],
  now: Date = new Date(),
): ComponentHealth {
  const incident = activeIncidentFor(component, incidents, now);
  if (incident) {
    return {
      ...component,
      state: SEVERITY_TO_STATE[incident.severity],
      incident,
    };
  }

  const maint = activeMaintenanceFor(component, maintenance, now);
  if (maint) {
    return { ...component, state: "maintenance", maintenance: maint };
  }

  return {
    ...component,
    state: component.probe.state,
    maintenanceScheduled: scheduledMaintenanceFor(component, maintenance, now) !== undefined,
  };
}

export interface SubsystemHealth {
  subsystem: Subsystem;
  /** Worst non-maintenance state of its members. */
  state: ComponentState;
  components: ComponentHealth[];
}

/** Worst state of the group; maintenance members do not degrade the group. */
function worstState(states: ComponentState[]): ComponentState {
  let worst: ComponentState = "operational";
  for (const s of states) {
    const rank = STATE_RANK[s];
    if (rank > STATE_RANK[worst]) worst = s;
  }
  return worst;
}

export type GlobalHealth = {
  state: ComponentState;
  subsystems: SubsystemHealth[];
  components: ComponentHealth[];
};

/**
 * Roll up all components into subsystems and a global state.
 * Provider subsystems (channel providers) are capped at `degraded`
 * for the global indicator: provider incidents are operational events,
 * not platform outages (design §9.2).
 */
export function resolveGlobalHealth(
  components: Component[],
  incidents: Incident[],
  maintenance: Maintenance[],
  now: Date = new Date(),
): GlobalHealth {
  const all = components.map((c) => resolveComponentHealth(c, incidents, maintenance, now));

  const groups = new Map<Subsystem, ComponentHealth[]>();
  for (const c of all) {
    const list = groups.get(c.subsystem) ?? [];
    list.push(c);
    groups.set(c.subsystem, list);
  }

  const subsystems: SubsystemHealth[] = [...groups.entries()].map(([subsystem, members]) => ({
    subsystem,
    state: worstState(members.map((m) => m.state)),
    components: members,
  }));

  let state: ComponentState = worstState(subsystems.map((s) => s.state));
  if (state === "major_outage" || state === "partial_outage") {
    const hasPlatformIssue = subsystems.some((s) => s.subsystem !== "providers" && STATE_RANK[s.state] > 1);
    if (!hasPlatformIssue) state = "degraded";
  }

  return { state, subsystems, components: all };
}

/** Global uptime % for a window, averaged over component availability. */
export function globalAvailability(
  components: Component[],
  days: "7d" | "30d" | "90d",
): number {
  if (components.length === 0) return 100;
  const sum = components.reduce((acc, c) => acc + c.probe.availability[days], 0);
  return Math.round((sum / components.length) * 100) / 100;
}
