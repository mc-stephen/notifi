export const env = {
  docs: (path = "") =>
    `${process.env.NEXT_PUBLIC_DOCS_URL ?? "https://docs.notifi.dev"}${path}`,
  apiBase: process.env.NEXT_PUBLIC_API_URL ?? "https://api.notifi.dev",
  status: process.env.NEXT_PUBLIC_STATUS_URL ?? "https://status.notifi.dev",
  github: process.env.NEXT_PUBLIC_GITHUB_URL ?? "https://github.com/notifi",
};
