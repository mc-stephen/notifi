import rss from "@astrojs/rss";
import { getIncidents, getMaintenance } from "../lib/data";
import { buildFeedEntries, filterEntries } from "../lib/feeds";

export async function GET(context: { site: URL; url: URL }) {
  const [incidents, maintenance] = await Promise.all([getIncidents(), getMaintenance()]);
  const incidentFilter = context.url.searchParams.get("incident");
  const site = context.site.toString().replace(/\/$/, "");

  const entries = filterEntries(buildFeedEntries(incidents, maintenance, site), incidentFilter);

  return rss({
    title: incidentFilter ? `Notifi Status — Incident updates` : "Notifi Status",
    description: "Current and historical health of the Notifi notification platform: incidents and scheduled maintenance.",
    site: context.site,
    items: entries.map((e) => ({
      title: e.title,
      link: e.link,
      pubDate: e.pubDate,
      author: e.author,
      categories: e.categories,
      description: e.content,
    })),
    customData: [
      `<language>en</language>`,
      `<managingEditor>status@notifi.dev (Notifi SRE)</managingEditor>`,
      `<lastBuildDate>${new Date().toUTCString()}</lastBuildDate>`,
      incidentFilter ? `<category>incident</category>` : "",
    ].join(""),
  });
}
