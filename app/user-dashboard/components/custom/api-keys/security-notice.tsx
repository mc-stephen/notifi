import { ShieldCheck } from "lucide-react";

export function ApiKeySecurityNotice() {
  return (
    <div className="flex items-start gap-3 rounded-lg border border-amber-500/20 bg-amber-500/5 px-4 py-3">
      <ShieldCheck className="mt-0.5 size-4 shrink-0 text-amber-600 dark:text-amber-400" />
      <div className="space-y-1">
        <p className="text-sm font-medium text-amber-600 dark:text-amber-400">
          Keep your API keys safe
        </p>
        <ul className="list-disc space-y-0.5 pl-5 text-sm text-muted-foreground">
          <li>
            Secrets are shown exactly once — store them in environment
            variables or a secret manager, never commit them.
          </li>
          <li>
            Use keys server-side only — never ship them in frontend or mobile
            bundles.
          </li>
          <li>
            Apply least privilege — keep development and production keys
            separate and prefer read-only where possible.
          </li>
          <li>
            Restrict each key with an IP allowlist, rotate regularly, and
            revoke keys you no longer use.
          </li>
        </ul>
      </div>
    </div>
  );
}
