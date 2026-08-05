import { getIncidents, getMaintenance } from "../lib/data";
import { buildFeedEntries, filterEntries } from "../lib/feeds";

function esc(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

export async function GET(context: { site: URL; url: URL }) {
  const [incidents, maintenance] = await Promise.all([getIncidents(), getMaintenance()]);
  const incidentFilter = context.url.searchParams.get("incident");
  const site = context.site.toString().replace(/\/$/, "");
  const now = new Date();

  const entries = filterEntries(buildFeedEntries(incidents, maintenance, site, now), incidentFilter);

  const itemXml = entries
    .map((e) => {
      const categories = e.categories
        .map((c) => `<category term="${esc(c)}" />`)
        .join("");
      return [
        `<entry>`,
        `<title>${esc(e.title)}</title>`,
        `<id>${esc(e.guid)}</id>`,
        `<link rel="alternate" href="${esc(e.link)}" />`,
        `<published>${new Date(e.pubDate).toISOString()}</published>`,
        `<updated>${new Date(e.updatedAt).toISOString()}</updated>`,
        `<author><name>${esc(e.author)}</name></author>`,
        categories,
        `<summary>${esc(e.content)}</summary>`,
        `</entry>`,
      ].join("");
    })
    .join("\n");

  const xml = [
    `<?xml version="1.0" encoding="utf-8"?>`,
    `<feed xmlns="http://www.w3.org/2005/Atom">`,
    `<title>${incidentFilter ? "Notifi Status — Incident updates" : "Notifi Status"}</title>`,
    `<id>${site}/</id>`,
    `<link href="${site}/atom.xml" rel="self" />`,
    `<updated>${now.toISOString()}</updated>`,
    `<author><name>Notifi SRE</name><email>status@notifi.dev</email></author>`,
    itemXml,
    `</feed>`,
  ].join("\n");

  return new Response(xml, {
    headers: { "Content-Type": "application/atom+xml; charset=utf-8" },
  });
}
