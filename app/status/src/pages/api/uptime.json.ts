import { getIncidents } from "../../lib/data";
import { rangeAvailability, uptimeSeries, uptimeSummary } from "../../lib/uptime";
import { jsonResponse } from "../../lib/api";

export async function GET() {
  const incidents = await getIncidents();
  return jsonResponse({
    range: "90d",
    unit: "day",
    summary: uptimeSummary(incidents),
    series: uptimeSeries("90d", incidents).map((p) => ({
      t: p.t,
      availability: p.availability,
    })),
  });
}
