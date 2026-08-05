import { getComponents, getIncidents, getMaintenance } from "../../lib/data";
import { resolveGlobalHealth } from "../../lib/status";
import { jsonResponse, serializeComponentState } from "../../lib/api";

export async function GET() {
  const [components, incidents, maintenance] = await Promise.all([getComponents(), getIncidents(), getMaintenance()]);
  const global = resolveGlobalHealth(components, incidents, maintenance);
  return jsonResponse(global.components.map(serializeComponentState));
}
