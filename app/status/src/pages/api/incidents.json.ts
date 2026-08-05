import { getIncidents } from "../../lib/data";
import { resolvedIncidents } from "../../lib/status";
import { jsonResponse, serializeIncidentSummary } from "../../lib/api";

export async function GET(context: { site: URL }) {
  const incidents = await getIncidents();
  const site = context.site.toString().replace(/\/$/, "");
  return jsonResponse(incidents.map((i) => serializeIncidentSummary(i, site)));
}
