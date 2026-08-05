import { getIncidents } from "../../../lib/data";
import { jsonResponse, serializeIncidentDetail } from "../../../lib/api";

export async function getStaticPaths() {
  const incidents = await getIncidents();
  return incidents.map((incident) => ({
    params: { id: incident.slug },
  }));
}

export async function GET({ params, site }: { params: { id?: string }; site: URL }) {
  const incidents = await getIncidents();
  const incident = incidents.find((i) => i.slug === params.id);

  if (!incident) {
    return new Response(JSON.stringify({ error: "not_found", detail: `No incident with id "${params.id}"` }), {
      status: 404,
      headers: { "Content-Type": "application/json; charset=utf-8" },
    });
  }

  const siteUrl = site.toString().replace(/\/$/, "");
  return jsonResponse(serializeIncidentDetail(incident, siteUrl));
}
