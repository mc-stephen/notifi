import rss from "@astrojs/rss";
import { getCollection } from "astro:content";
import { SITE_NAME, SITE_DESCRIPTION } from "../lib/seo";

export async function GET(context: { site: URL }) {
  const changelog = (await getCollection("changelog")).sort(
    (a, b) => b.data.date.valueOf() - a.data.date.valueOf()
  );

  const items = changelog.map((entry) => ({
    title: entry.data.title,
    pubDate: entry.data.date,
    description: entry.data.changes
      .map((change) => `${change.type}: ${change.detail}`)
      .join(" — "),
    link: `/#changelog-${entry.data.slug}`,
  }));

  return rss({
    title: `${SITE_NAME} — Changelog`,
    description: SITE_DESCRIPTION,
    site: context.site,
    items,
    customData: `<language>en-us</language>`,
  });
}
