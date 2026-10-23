"use client";

import type { CSSProperties, ReactNode } from "react";
import { Handle, Position, type NodeProps } from "@xyflow/react";
import { FileText, Globe, OctagonX, Send, Sparkles, Split, Webhook, Zap } from "lucide-react";
import { cn } from "@/lib/utils";
import { HANDLE, type FlowNode, type FlowNodeKind } from "../flow-types";

function Shell({
  selected,
  icon,
  title,
  subtitle,
  children,
  hasError,
}: {
  selected?: boolean;
  icon: ReactNode;
  title: string;
  subtitle: string;
  children?: ReactNode;
  hasError?: boolean;
}) {
  return (
    <div
      className={cn(
        "w-64 rounded-lg border bg-popover text-foreground shadow-xl transition-colors dark:bg-[#17171c] dark:text-zinc-100",
        selected ? "border-violet-400/70" : "border-border dark:border-white/10",
        hasError && "border-destructive",
      )}
    >
      <div className="flex items-center gap-2 border-b border-border px-3 py-2 dark:border-white/10">
        <span className="flex size-6 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground dark:bg-white/10 dark:text-zinc-200">
          {icon}
        </span>
        <div className="min-w-0">
          <p className="truncate text-[13px] font-semibold">{title}</p>
          <p className="truncate text-[11px] text-muted-foreground dark:text-zinc-400">{subtitle}</p>
        </div>
      </div>
      {children}
    </div>
  );
}

function Port({
  type,
  position,
  id,
  label,
  color,
  top,
}: {
  type: "source" | "target";
  position: Position;
  id: string;
  label?: string;
  /** Hex fill — applied via xyflow CSS vars + inline style, which always win. */
  color: string;
  /** Vertical anchor override (default centers on the node edge). */
  top?: string;
}) {
  const style: CSSProperties = {
    width: 12,
    height: 12,
    borderWidth: 2,
    // Matches the node face in both modes (dark face is near-popover).
    borderColor: "var(--popover)",
    ["--xy-handle-background-color" as string]: color,
    ...(top ? { top } : {}),
  };
  return <Handle type={type} position={position} id={id} title={label ?? id} style={style} />;
}

type NodeComponentProps = NodeProps<FlowNode> & {
  data: FlowNode["data"];
  selected?: boolean;
};

export function TriggerNode({ data, selected }: NodeComponentProps) {
  const kind = "kind" in data && (data.kind === "webhook" || data.kind === "event") ? data.kind : "event";
  const preset = "preset" in data && typeof data.preset === "string" && data.preset ? data.preset : null;
  const event = "event" in data && typeof data.event === "string" ? data.event : "notification.created";
  const note = "note" in data && typeof data.note === "string" ? data.note : "";
  const subtitle =
    kind === "webhook" ? "Inbound webhook" : preset ? `${preset.replace(/s$/, "")} event` : "Starts the flow";
  return (
    <Shell selected={selected} icon={kind === "webhook" ? <Webhook className="size-3.5" /> : <Zap className="size-3.5" />} title="Trigger" subtitle={subtitle}>
      <div className="px-3 py-2">
        <p className="truncate font-mono text-[11px] text-muted-foreground dark:text-zinc-300">
          {kind === "webhook" ? (note || "POST to fire") : event}
        </p>
      </div>
      <Port type="source" position={Position.Right} id={HANDLE.out} label="out" color="#34d399" />
    </Shell>
  );
}

export function ProviderNode({ data, selected }: NodeComponentProps) {
  const providerId = "providerId" in data && typeof data.providerId === "string" ? data.providerId : "";
  const priority = "priority" in data && typeof data.priority === "number" ? data.priority : 1;
  const channelId = "channelId" in data && typeof data.channelId === "string" ? data.channelId : "";
  return (
    <Shell selected={selected} icon={<Send className="size-3.5" />} title="Send via provider" subtitle={channelId || "Pick a channel"}>
      <div className="space-y-1 px-3 py-2">
        <p className="truncate text-[12px] font-medium text-foreground dark:text-zinc-100">
          {providerId || "Select provider"}
        </p>
        <p className="text-[11px] text-muted-foreground dark:text-zinc-400">Priority {priority}</p>
      </div>
      <Port type="target" position={Position.Left} id={HANDLE.in} label="in" color="#a78bfa" />
      <span className="pointer-events-none absolute top-[38%] right-0 flex translate-x-1/2 items-center gap-1">
        <span className="text-[10px] text-emerald-600 dark:text-emerald-300/80">ok</span>
      </span>
      <Port type="source" position={Position.Right} id={HANDLE.success} label="on success" color="#34d399" top="38%" />
      <span className="pointer-events-none absolute top-[62%] right-0 flex translate-x-1/2 items-center gap-1">
        <span className="text-[10px] text-amber-600 dark:text-amber-300/80">fail</span>
      </span>
      <Port type="source" position={Position.Right} id={HANDLE.failure} label="on failure" color="#fbbf24" top="62%" />
    </Shell>
  );
}

export function ConditionNode({ data }: NodeComponentProps) {
  const field = "field" in data && typeof data.field === "string" ? data.field : "";
  const operator = "operator" in data && typeof data.operator === "string" ? data.operator : "equals";
  const value = "value" in data && typeof data.value === "string" ? data.value : "";
  return (
    <Shell icon={<Split className="size-3.5" />} title="Condition" subtitle="Branch the flow">
      <div className="px-3 py-2">
        <p className="truncate font-mono text-[11px] text-muted-foreground dark:text-zinc-300">
          {field ? `${field} ${operator} ${value || "…"}` : "Build a rule"}
        </p>
      </div>
      <Port type="target" position={Position.Left} id={HANDLE.in} label="in" color="#a78bfa" />
      <span className="pointer-events-none absolute top-[38%] right-0 flex translate-x-1/2 items-center gap-1">
        <span className="text-[10px] text-emerald-600 dark:text-emerald-300/80">yes</span>
      </span>
      <Port type="source" position={Position.Right} id={HANDLE.yes} label="yes" color="#34d399" top="38%" />
      <span className="pointer-events-none absolute top-[62%] right-0 flex translate-x-1/2 items-center gap-1">
        <span className="text-[10px] text-rose-300/80">no</span>
      </span>
      <Port type="source" position={Position.Right} id={HANDLE.no} label="no" color="#fb7185" top="62%" />
    </Shell>
  );
}

export function AiTransformNode({ data }: NodeComponentProps) {
  const model = "model" in data && typeof data.model === "string" ? data.model : "auto";
  const instruction = "instruction" in data && typeof data.instruction === "string" ? data.instruction : "";
  return (
    <Shell icon={<Sparkles className="size-3.5" />} title="AI transform" subtitle={`Model: ${model}`}>
      <div className="px-3 py-2">
        <p className="line-clamp-2 text-[11px] text-muted-foreground dark:text-zinc-300">
          {instruction || "Describe the transformation…"}
        </p>
      </div>
      <Port type="target" position={Position.Left} id={HANDLE.in} label="in" color="#a78bfa" />
      <Port type="source" position={Position.Right} id={HANDLE.out} label="out" color="#34d399" />
    </Shell>
  );
}

export function TemplateNode({ data, selected }: NodeComponentProps) {
  const templateId = "templateId" in data && typeof data.templateId === "string" ? data.templateId : "";
  const templateName = "templateName" in data && typeof data.templateName === "string" ? data.templateName : "";
  const picked = templateId !== "";
  return (
    <Shell selected={selected} icon={<FileText className="size-3.5" />} title="Template" subtitle={picked ? "Content source" : "Message content"}>
      <div className="space-y-1 px-3 py-2">
        <p className="truncate text-[12px] font-medium text-foreground dark:text-zinc-100">
          {picked ? templateName || templateId : "Select a template"}
        </p>
        <p className="text-[11px] text-muted-foreground dark:text-zinc-400">Rendered fresh at every fire</p>
      </div>
      <Port type="target" position={Position.Left} id={HANDLE.in} label="in" color="#a78bfa" />
      <span className="pointer-events-none absolute top-1/2 right-0 flex translate-x-1/2 items-center gap-1">
        <span className="text-[10px] text-emerald-600 dark:text-emerald-300/80">out</span>
      </span>
      <Port type="source" position={Position.Right} id={HANDLE.out} label="out" color="#34d399" top="50%" />
    </Shell>
  );
}

export function HttpRequestNode({ data, selected }: NodeComponentProps) {
  const url = "url" in data && typeof data.url === "string" ? data.url : "";
  const method = "method" in data && typeof data.method === "string" ? data.method : "POST";
  const host = (() => {
    try {
      return url ? new URL(url).host : "";
    } catch {
      return "";
    }
  })();
  return (
    <Shell selected={selected} icon={<Globe className="size-3.5" />} title="HTTP Request" subtitle={method}>
      <div className="space-y-1 px-3 py-2">
        <p className="truncate font-mono text-[11px] text-muted-foreground dark:text-zinc-300">
          {host || url || "Set a URL"}
        </p>
      </div>
      <Port type="target" position={Position.Left} id={HANDLE.in} label="in" color="#a78bfa" />
      <span className="pointer-events-none absolute top-[38%] right-0 flex translate-x-1/2 items-center gap-1">
        <span className="text-[10px] text-emerald-600 dark:text-emerald-300/80">ok</span>
      </span>
      <Port type="source" position={Position.Right} id={HANDLE.success} label="on success" color="#34d399" top="38%" />
      <span className="pointer-events-none absolute top-[62%] right-0 flex translate-x-1/2 items-center gap-1">
        <span className="text-[10px] text-amber-600 dark:text-amber-300/80">fail</span>
      </span>
      <Port type="source" position={Position.Right} id={HANDLE.failure} label="on failure" color="#fbbf24" top="62%" />
    </Shell>
  );
}

export function StopNode({ data }: NodeComponentProps) {  const reason = "reason" in data && typeof data.reason === "string" ? data.reason : "";
  return (
    <Shell icon={<OctagonX className="size-3.5" />} title="Stop" subtitle={reason || "End of flow"}>
      <Port type="target" position={Position.Left} id={HANDLE.in} label="in" color="#a78bfa" />
    </Shell>
  );
}

export const FLOW_NODE_TYPES: Record<FlowNodeKind, React.ComponentType<NodeComponentProps>> = {
  trigger: TriggerNode,
  provider: ProviderNode,
  template: TemplateNode,
  "http-request": HttpRequestNode,
  condition: ConditionNode,
  "ai-transform": AiTransformNode,
  stop: StopNode,
};
