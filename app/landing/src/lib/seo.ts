const SITE_NAME = "Notifi";
const SITE_DESCRIPTION =
  "One API. Every notification. Every platform. Send email, SMS, push, and desktop notifications from a single endpoint — with retries, analytics, and enterprise reliability.";

export { SITE_NAME, SITE_DESCRIPTION };

export function canonicalUrl(path = "/", site = "https://notifi.dev"): string {
  return new URL(path, site).href;
}

export function seoTitle(title: string, suffix = true): string {
  return suffix ? `${title} · ${SITE_NAME}` : title;
}

export function ogUrl(path = "/"): string {
  return canonicalUrl(path);
}

export function ogImage(path = "/og/home.svg"): string {
  return canonicalUrl(path);
}
