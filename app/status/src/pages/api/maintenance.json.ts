import { getMaintenance } from "../../lib/data";
import { jsonResponse, serializeMaintenance } from "../../lib/api";

export async function GET(context: { site: URL }) {
  const maintenance = await getMaintenance();
  const site = context.site.toString().replace(/\/$/, "");
  return jsonResponse(maintenance.map((m) => serializeMaintenance(m, site)));
}
