// @ts-check
import { defineConfig } from "astro/config";
import mdx from "@astrojs/mdx";
import sitemap from "@astrojs/sitemap";

// https://astro.build/config
export default defineConfig({
  site: "https://notifi.dev",
  trailingSlash: "never",
  output: "static",
  integrations: [
    mdx(),
    sitemap({
      filter: (page) => !page.endsWith("/404"),
    }),
  ],
});