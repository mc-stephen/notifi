import { getIncidents } from "../../../lib/data";
import { rangeAvailability, uptimeSeries, UPTIME_RANGES, type UptimeRange } from "../../../lib/uptime";
import { jsonResponse } from "../../../lib/api";

export async function getStaticPaths() {
  return UPTIME_RANGES.map((range) => ({ params: { range } }));
}

export async function GET({ params }: { params: { range?: string } }) {
  const range = params.range as UptimeRange | undefined;

  if (!range || !(UPTIME_RANGES as readonly string[]).includes(range)) {
    return new Response(JSON.stringify({ error: "not_found", detail: `Unknown range "${range}". Valid: ${UPTIME_RANGES.join(", ")}` }), {
      status: 404,
      headers: { "Content-Type": "application/json; charset=utf-8" },
    });
  }

  const incidents = await getIncidents();
  return jsonResponse({
    range,
    unit: range === "24h" || range === "7d" ? "hour" : range === "1y" ? "week" : "day",
    availability: rangeAvailability(range, incidents),
    series: uptimeSeries(range, incidents).map((p) => ({
      t: p.t,
      availability: p.availability,
    })),
  });
}
