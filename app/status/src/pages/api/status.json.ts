import { getComponents, getIncidents, getMaintenance } from "../../lib/data";
import { resolveGlobalHealth } from "../../lib/status";
import { rangeAvailability } from "../../lib/uptime";
import { jsonResponse, serializeStatus } from "../../lib/api";

export async function GET() {
  const [components, incidents, maintenance] = await Promise.all([getComponents(), getIncidents(), getMaintenance()]);
  const global = resolveGlobalHealth(components, incidents, maintenance);
  const uptime90d = rangeAvailability("90d", incidents);
  return jsonResponse(serializeStatus(global, uptime90d));
}
