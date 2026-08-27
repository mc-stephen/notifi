const WEBSITE_URL =
  process.env.NEXT_PUBLIC_WEBSITE_URL ?? "https://notifi.dev";

export const env = {
  docs: (path = "") =>
    `${process.env.NEXT_PUBLIC_DOCS_URL ?? "https://docs.notifi.dev"}${path}`,
  apiBase: process.env.NEXT_PUBLIC_API_URL ?? "https://api.notifi.dev",
  status: process.env.NEXT_PUBLIC_STATUS_URL ?? "https://status.notifi.dev",
  github: process.env.NEXT_PUBLIC_GITHUB_URL ?? "https://github.com/notifi",
  website: (path = "") => `${WEBSITE_URL}${path}`,
};

/**
 * Marketing-site destinations — the ONLY place these paths are spelled out.
 * Components import from here so a path change (e.g. /privacy →
 * /legal/privacy) is a one-line edit.
 */
export const links = {
  terms: env.website("/terms"),
  privacy: env.website("/privacy"),
};
