import defaultMdxComponents from 'fumadocs-ui/mdx';
import { GetMethod, PostMethod, PutMethod, DeleteMethod } from './method-badge';

type SDK = 'javascript' | 'typescript' | 'python' | 'go' | 'rust' | 'java';

const sdkColors: Record<SDK, string> = {
  javascript: 'bg-yellow-50 text-yellow-700 ring-yellow-600/20 dark:bg-yellow-950 dark:text-yellow-300 dark:ring-yellow-400/20',
  typescript: 'bg-blue-50 text-blue-700 ring-blue-600/20 dark:bg-blue-950 dark:text-blue-300 dark:ring-blue-400/20',
  python: 'bg-sky-50 text-sky-700 ring-sky-600/20 dark:bg-sky-950 dark:text-sky-300 dark:ring-sky-400/20',
  go: 'bg-cyan-50 text-cyan-700 ring-cyan-600/20 dark:bg-cyan-950 dark:text-cyan-300 dark:ring-cyan-400/20',
  rust: 'bg-orange-50 text-orange-700 ring-orange-600/20 dark:bg-orange-950 dark:text-orange-300 dark:ring-orange-400/20',
  java: 'bg-red-50 text-red-700 ring-red-600/20 dark:bg-red-950 dark:text-red-300 dark:ring-red-400/20',
};

export function SDKBadge({ sdk }: { sdk: SDK }) {
  return (
    <span
      className={`inline-flex items-center rounded-md px-2 py-0.5 text-xs font-semibold ring-1 ${sdkColors[sdk]}`}
    >
      {sdk.charAt(0).toUpperCase() + sdk.slice(1)}
    </span>
  );
}

export function EndpointCard({ method, path, description }: { method: string; path: string; description?: string }) {
  const methodColors: Record<string, string> = {
    GET: 'bg-emerald-50 text-emerald-700 ring-emerald-600/20 dark:bg-emerald-950 dark:text-emerald-300 dark:ring-emerald-400/20',
    POST: 'bg-blue-50 text-blue-700 ring-blue-600/20 dark:bg-blue-950 dark:text-blue-300 dark:ring-blue-400/20',
    PUT: 'bg-amber-50 text-amber-700 ring-amber-600/20 dark:bg-amber-950 dark:text-amber-300 dark:ring-amber-400/20',
    DELETE: 'bg-red-50 text-red-700 ring-red-600/20 dark:bg-red-950 dark:text-red-300 dark:ring-red-400/20',
  };
  return (
    <div className="not-prose mb-4 flex items-center gap-3 rounded-lg border border-fd-border bg-fd-card p-3">
      <span className={`inline-flex items-center rounded-md px-2 py-0.5 text-xs font-semibold font-mono ring-1 ${methodColors[method] || methodColors.GET}`}>
        {method}
      </span>
      <code className="text-sm font-mono text-fd-foreground">{path}</code>
      {description && <span className="ml-auto text-xs text-fd-muted-foreground">{description}</span>}
    </div>
  );
}

export function StatusLabel({ code, label }: { code: number; label: string }) {
  const colorMap: Record<string, string> = {
    '2': 'bg-emerald-50 text-emerald-700 ring-emerald-600/20 dark:bg-emerald-950 dark:text-emerald-300 dark:ring-emerald-400/20',
    '4': 'bg-amber-50 text-amber-700 ring-amber-600/20 dark:bg-amber-950 dark:text-amber-300 dark:ring-amber-400/20',
    '5': 'bg-red-50 text-red-700 ring-red-600/20 dark:bg-red-950 dark:text-red-300 dark:ring-red-400/20',
  };
  const prefix = String(code)[0];
  const colors = colorMap[prefix] || colorMap['2'];
  return (
    <span className={`inline-flex items-center gap-1.5 rounded-md px-2 py-0.5 text-xs font-mono font-semibold ring-1 ${colors}`}>
      <span>{code}</span>
      <span className="font-sans font-normal">{label}</span>
    </span>
  );
}

export function VersionNotice({ version, type = 'new' }: { version: string; type?: 'new' | 'deprecated' | 'breaking' }) {
  const styles = {
    new: 'bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-950 dark:text-blue-300 dark:border-blue-800',
    deprecated: 'bg-amber-50 text-amber-700 border-amber-200 dark:bg-amber-950 dark:text-amber-300 dark:border-amber-800',
    breaking: 'bg-red-50 text-red-700 border-red-200 dark:bg-red-950 dark:text-red-300 dark:border-red-800',
  };
  const labels = { new: 'New', deprecated: 'Deprecated', breaking: 'Breaking' };
  return (
    <div className={`not-prose mb-4 rounded-lg border px-4 py-3 text-sm ${styles[type]}`}>
      <strong className="font-semibold">{labels[type]} in {version}</strong>
    </div>
  );
}

export const mdxComponents = {
  ...defaultMdxComponents,
  GetMethod,
  PostMethod,
  PutMethod,
  DeleteMethod,
  SDKBadge,
  EndpointCard,
  StatusLabel,
  VersionNotice,
};
