"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Check, Copy, ChevronRight, ChevronDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";

type JsonViewerProps = {
  data: unknown;
  className?: string;
  defaultExpanded?: boolean;
  showCopy?: boolean;
};

function formatJson(data: unknown): string {
  return JSON.stringify(data, null, 2);
}

function JsonValue({ value, expanded }: { value: unknown; expanded: boolean }) {
  if (value === null) return <span className="text-muted-foreground italic">null</span>;
  if (value === undefined) return <span className="text-muted-foreground italic">undefined</span>;
  if (typeof value === "boolean") return <span className="text-chart-4">{String(value)}</span>;
  if (typeof value === "number") return <span className="text-chart-2">{value}</span>;
  if (typeof value === "string") return <span className="text-success">&quot;{value}&quot;</span>;
  if (Array.isArray(value)) {
    if (!expanded) return <span className="text-muted-foreground">[{value.length} items]</span>;
    return <span className="text-muted-foreground">Array({value.length})</span>;
  }
  if (typeof value === "object") {
    const keys = Object.keys(value as Record<string, unknown>);
    if (!expanded) return <span className="text-muted-foreground">{'{'}...{'}'}</span>;
    return <span className="text-muted-foreground">Object({'{'}{keys.length} keys{'}'})</span>;
  }
  return <span>{String(value)}</span>;
}

function JsonNode({ keyName, value, depth = 0, defaultExpanded = true }: { keyName: string; value: unknown; depth?: number; defaultExpanded?: boolean }) {
  const isExpandable = value !== null && typeof value === "object";
  const [expanded, setExpanded] = useState(depth < 2 ? defaultExpanded : false);

  if (!isExpandable) {
    return (
      <div className="flex items-baseline gap-2 font-mono text-sm" style={{ paddingLeft: depth * 16 }}>
        <span className="text-muted-foreground">{keyName}:</span>
        <JsonValue value={value} expanded={false} />
      </div>
    );
  }

  const entries = Array.isArray(value)
    ? value.map((v, i) => [String(i), v] as const)
    : Object.entries(value as Record<string, unknown>);

  return (
    <div>
      <button
        className="flex items-baseline gap-1 font-mono text-sm hover:bg-muted/50 rounded px-1 -mx-1"
        style={{ paddingLeft: depth * 16 }}
        onClick={() => setExpanded(!expanded)}
      >
        {expanded ? <ChevronDown className="size-3 mt-0.5 text-muted-foreground shrink-0" /> : <ChevronRight className="size-3 mt-0.5 text-muted-foreground shrink-0" />}
        <span className="text-muted-foreground">{keyName}:</span>
        <JsonValue value={value} expanded={expanded} />
      </button>
      {expanded && (
        <div className="border-l ml-[calc(var(--depth)*16px+6px)]" style={{ '--depth': depth } as React.CSSProperties}>
          {entries.map(([k, v]) => (
            <JsonNode key={k} keyName={k} value={v} depth={depth + 1} defaultExpanded={defaultExpanded} />
          ))}
        </div>
      )}
    </div>
  );
}

export function JsonViewer({ data, className, defaultExpanded = true, showCopy = true }: JsonViewerProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(formatJson(data));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  if (data === null || data === undefined) {
    return (
      <div className={cn("rounded-lg border bg-muted/50 p-4 text-sm font-mono text-muted-foreground italic", className)}>
        null
      </div>
    );
  }

  if (typeof data !== "object") {
    return (
      <div className={cn("rounded-lg border bg-muted/50 p-4 text-sm font-mono", className)}>
        {typeof data === "string" ? `&quot;${data}&quot;` : String(data)}
      </div>
    );
  }

  return (
    <div className={cn("relative group rounded-lg border bg-muted/50 p-4", className)}>
      <div className="space-y-0.5">
        {Array.isArray(data)
          ? data.map((item, i) => <JsonNode key={i} keyName={String(i)} value={item} defaultExpanded={defaultExpanded} />)
          : Object.entries(data as Record<string, unknown>).map(([k, v]) => (
              <JsonNode key={k} keyName={k} value={v} defaultExpanded={defaultExpanded} />
            ))}
      </div>
      {showCopy && (
        <Tooltip>
          <TooltipTrigger
            render={
              <Button
                variant="ghost"
                size="icon-xs"
                className="absolute right-2 top-2 opacity-0 transition-opacity group-hover:opacity-100"
                onClick={handleCopy}
              />
            }
          >
            {copied ? <Check className="size-3" /> : <Copy className="size-3" />}
          </TooltipTrigger>
          <TooltipContent>{copied ? "Copied" : "Copy"}</TooltipContent>
        </Tooltip>
      )}
    </div>
  );
}
