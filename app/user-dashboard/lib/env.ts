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
 * Single source of truth for every link leaving the dashboard. Paths are
 * spelled out ONLY here — components import from `links`, never build URLs
 * themselves, so a path change (e.g. /privacy → /legal/privacy) is a
 * one-line edit.
 */
export const links = {
  // marketing site
  terms: env.website("/terms"),
  privacy: env.website("/privacy"),

  // docs
  docs: env.docs(),
  docsApi: env.docs("/api"),
  docsGettingStarted: env.docs("/getting-started"),
  docsSdks: env.docs("/sdks"),
  docsChangelog: env.docs("/changelog"),
  sdkDocs: {
    node: env.docs("/sdks/node"),
    python: env.docs("/sdks/python"),
    go: env.docs("/sdks/go"),
    rust: env.docs("/sdks/rust"),
    "react-native": env.docs("/sdks/react-native"),
    flutter: env.docs("/sdks/flutter"),
    swift: env.docs("/sdks/swift"),
    kotlin: env.docs("/sdks/kotlin"),
  },
  integrationDocs: {
    segment: env.docs("/integrations/segment"),
    posthog: env.docs("/integrations/posthog"),
    datadog: env.docs("/integrations/datadog"),
    sentry: env.docs("/integrations/sentry"),
    zapier: env.docs("/integrations/zapier"),
    mixpanel: env.docs("/integrations/mixpanel"),
    slack: env.docs("/integrations/slack"),
    pagerduty: env.docs("/integrations/pagerduty"),
    opentelemetry: env.docs("/integrations/opentelemetry"),
  },
};
