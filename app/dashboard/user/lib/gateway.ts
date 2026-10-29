/**
 * Linux Notifi Gateway helpers: the route-secret generator and the
 * subscribe-URL builder (the URL the user's Linux app polls).
 */

/** URL-safe secret alphabet (base64url without padding). */
function toUrlSafeBase64(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

/**
 * Generates a 32-byte route secret (43 url-safe characters) for a fresh
 * gateway connect. Event-handler only: never called while rendering, so
 * the server render and hydration stay identical.
 */
export function generateRouteSecret(): string {
  const bytes = new Uint8Array(32);
  crypto.getRandomValues(bytes);
  return toUrlSafeBase64(bytes);
}

/** The poll URL for one secret route (`?wait=N` long-polls on the app side). */
export function gatewaySubscribeUrl(
  apiBase: string,
  topic: string,
  secretKey: string,
): string {
  return `${apiBase}/gateway/linux/${topic}?key=${secretKey}`;
}

/** Friendly labels for the mode dropdown's saved option ids. */
export const GATEWAY_MODE_LABELS: Record<string, string> = {
  polling: "Web polling (recommended)",
  https: "HTTPS check",
};
