import { codeToHtml, createCssVariablesTheme } from "shiki";

/**
 * Build-time syntax highlighting for code samples. Uses Shiki's
 * css-variables theme so token colors resolve against the design
 * tokens defined in global.css (dark + light via `data-theme`),
 * instead of shipping hardcoded theme palettes.
 */
const theme = createCssVariablesTheme({
  name: "notifi",
  variablePrefix: "--notifi-",
});

export async function highlight(code: string, lang: string): Promise<string> {
  const html = await codeToHtml(code, { lang, theme });
  const match = html.match(/^<pre[^>]*>\s*<code[^>]*>([\s\S]*)<\/code>\s*<\/pre>\s*$/);
  return match ? match[1] : escapeHtml(code);
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}
