"use client";

import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  Panel,
  ReactFlow,
  ReactFlowProvider,
  addEdge,
  useEdgesState,
  useNodesState,
  useReactFlow,
  type Connection,
  type Edge,
} from "@xyflow/react";
import { Button } from "@/components/ui/button";
import { CheckCircle2, FlaskConical, ListX, ShieldCheck, X } from "lucide-react";
import { useTheme } from "next-themes";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { WEBHOOK_EVENTS } from "@/lib/constants";
import { FLOW_NODE_TYPES } from "./nodes/flow-nodes";
import { FlowPalette, DRAG_MIME, TRIGGER_PRESET_MIME, PROVIDER_CHANNEL_MIME, type TriggerPreset } from "./FlowPalette";
import { NodeConfigPanel } from "./NodeConfigPanel";
import {
  HANDLE,
  newTriggerToken,
  type FlowDoc,
  type FlowIssue,
  type FlowNode,
  type FlowNodeKind,
} from "./flow-types";
import { validateFlow } from "./flow-validate";

function defaultDataFor(
  kind: FlowNodeKind,
  triggerPreset?: TriggerPreset,
  providerChannel?: string,
): FlowNode["data"] {
  switch (kind) {
    case "trigger": {
      if (triggerPreset === "webhook") {
        return { kind: "webhook", token: newTriggerToken(), note: "" };
      }
      const group = WEBHOOK_EVENTS.find((g) => g.label === triggerPreset);
      const event =
        triggerPreset === "Notifications" || !group
          ? "notification.created"
          : (group.events[0]?.key ?? "notification.created");
      return { kind: "event", preset: triggerPreset, event };
    }
    case "provider":
      return {
        channelId: providerChannel ?? "",
        providerId: "",
        priority: 1,
        preset: providerChannel,
      };
    case "template":
      return { templateId: "" };
    case "http-request":
      return { url: "", method: "POST", headers: [], body: "" };
    case "condition":
      return { field: "", operator: "equals", value: "" };
    case "ai-transform":
      return { model: "auto", instruction: "", useBrandWrapper: true };
    case "stop":
      return { reason: "" };
  }
}

/**
 * Persistence-agnostic flow editor. The parent owns loading (server doc)
 * and saving (server PUT); the canvas only edits and reports.
 * Remount via `key` when switching flows — state initializes once.
 *
 * Owner-managed flows (schedule/webhook-born) hide the trigger palette
 * section and lock their seeded trigger: it can be inspected but not
 * deleted or re-typed — the owner page owns it.
 */
export function FlowCanvas({
  initialDoc,
  onSave,
  actions,
  ownerManaged = false,
}: {
  initialDoc: FlowDoc;
  onSave: (doc: FlowDoc) => Promise<void>;
  /** Optional controls pinned to the canvas top-left panel (e.g. back). */
  actions?: React.ReactNode;
  /** Schedule/webhook flows: trigger is owned data. */
  ownerManaged?: boolean;
}) {
  return (
    <ReactFlowProvider>
      <FlowEditor
        initialDoc={initialDoc}
        onSave={onSave}
        actions={actions}
        ownerManaged={ownerManaged}
      />
    </ReactFlowProvider>
  );
}

function FlowEditor({
  initialDoc,
  onSave,
  actions,
  ownerManaged,
}: {
  initialDoc: FlowDoc;
  onSave: (doc: FlowDoc) => Promise<void>;
  actions?: React.ReactNode;
  ownerManaged: boolean;
}) {
  const [nodes, setNodes, onNodesChangeBase] = useNodesState(initialDoc.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialDoc.edges);
  // Owner-managed trigger is undeletable: strip its remove changes (the
  // drag-select + Backspace path) while leaving others untouched.
  const onNodesChangeSafe = useCallback(
    (changes: Parameters<typeof onNodesChangeBase>[0]) => {
      const triggerIds = new Set(
        nodes.filter((n) => n.type === "trigger").map((n) => n.id),
      );
      onNodesChangeBase(
        ownerManaged
          ? changes.filter(
              (c) => !("type" in c && c.type === "remove" && triggerIds.has(c.id)),
            )
          : changes,
      );
    },
    [ownerManaged, nodes, onNodesChangeBase],
  );
  // Panel state is explicit click state, not canvas selection state:
  // dragging a node may select it for keyboard delete, but must not
  // open its config panel.
  const [openId, setOpenId] = useState<string | null>(null);
  const [issues, setIssues] = useState<FlowIssue[]>([]);
  const [savedAt, setSavedAt] = useState<Date | null>(null);
  const idCounter = useRef(0);
  const { screenToFlowPosition } = useReactFlow();
  // JS-driven canvas colors (dots, minimap) follow the dashboard theme.
  // Mount-gated to avoid hydration mismatch; CSS-driven chrome uses
  // `dark:` variants and flips with the class, flash-free.
  const mounted = useSyncExternalStore(
    () => () => {},
    () => true,
    () => false,
  );
  const { resolvedTheme } = useTheme();
  const dark = mounted && resolvedTheme === "dark";

  // Autosave (500ms trailing). Skips the very first paint so opening a
  // flow never writes. Timestamp lands on success; failures toast.
  const firstPaint = useRef(true);
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingRef = useRef<FlowDoc | null>(null);
  const onSaveRef = useRef(onSave);
  useEffect(() => {
    onSaveRef.current = onSave;
  });
  useEffect(() => {
    if (firstPaint.current) {
      firstPaint.current = false;
      return;
    }
    const doc: FlowDoc = { version: 1, nodes, edges };
    pendingRef.current = doc;
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(() => {
      saveTimer.current = null;
      onSaveRef
        .current(doc)
        .then(() => {
          pendingRef.current = null;
          setSavedAt(new Date());
        })
        .catch(() => {
          toast.error("Couldn't save flow.");
        });
    }, 500);
  }, [nodes, edges]);

  // Flush on unmount.
  useEffect(() => {
    return () => {
      if (saveTimer.current) clearTimeout(saveTimer.current);
      const pending = pendingRef.current;
      pendingRef.current = null;
      if (pending) {
        onSaveRef.current?.(pending).catch(() => {});
      }
    };
  }, []);

  const onConnect = useCallback(
    (connection: Connection) => {
      setEdges((eds) => {
        const duplicate = eds.some(
          (e) =>
            e.source === connection.source &&
            e.target === connection.target &&
            (e.sourceHandle ?? null) === (connection.sourceHandle ?? null) &&
            (e.targetHandle ?? null) === (connection.targetHandle ?? null),
        );
        if (duplicate) return eds;
        return addEdge(
          {
            ...connection,
            id: `e-${Date.now().toString(36)}-${eds.length}`,
            style: { stroke: "#71717a", strokeWidth: 2 },
          },
          eds,
        );
      });
    },
    [setEdges],
  );

  const isValidConnection = useCallback((connection: Edge | Connection) => {
    if (connection.source === connection.target) return false;
    // Everything feedable exposes an `in` port; triggers have none, so
    // this also blocks wiring into a trigger.
    return connection.targetHandle === HANDLE.in;
  }, []);

  const onDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      const kind = e.dataTransfer.getData(DRAG_MIME) as FlowNodeKind | "";
      if (!kind) return;
      const triggerPreset = e.dataTransfer.getData(TRIGGER_PRESET_MIME) as TriggerPreset | "";
      const providerChannel = e.dataTransfer.getData(PROVIDER_CHANNEL_MIME) || undefined;
      idCounter.current += 1;
      const position = screenToFlowPosition({ x: e.clientX, y: e.clientY });
      const node: FlowNode = {
        id: `node-${Date.now().toString(36)}-${idCounter.current}`,
        type: kind,
        position: { x: position.x - 128, y: position.y - 40 },
        data: defaultDataFor(kind, triggerPreset || undefined, providerChannel),
      };
      setNodes((ns) => [...ns, node]);
      // Click-only panels: dropping a palette item places it without
      // opening its config panel. The user opens it with a click.
    },
    [screenToFlowPosition, setNodes],
  );

  const openNode = nodes.find((n) => n.id === openId) ?? null;

  const updateNodeData = useCallback(
    (data: FlowNode["data"]) => {
      if (!openId) return;
      setNodes((ns) => ns.map((n) => (n.id === openId ? { ...n, data } : n)));
    },
    [openId, setNodes],
  );

  const runValidation = useCallback(() => {
    const found = validateFlow(nodes, edges);
    setIssues(found);
    const errors = found.filter((i) => i.severity === "error");
    if (errors.length > 0) {
      toast.error(`Flow has ${errors.length} problem${errors.length === 1 ? "" : "s"} — see the list.`);
    }
    return found;
  }, [nodes, edges]);

  const runTest = useCallback(() => {
    const found = runValidation();
    if (found.filter((i) => i.severity === "error").length === 0) {
      toast.success(`Flow valid — ${nodes.length} nodes, ${edges.length} edges.`);
    }
  }, [runValidation, nodes.length, edges.length]);

  const errorIds = useMemo(() => new Set(issues.filter((i) => i.severity === "error" && i.nodeId).map((i) => i.nodeId!)), [issues]);

  // Stable identities keep React Flow from churning node state on every
  // render while validation outlines are applied.
  const displayNodes = useMemo(
    () =>
      nodes.map((n) =>
        errorIds.has(n.id)
          ? { ...n, style: { ...n.style, outline: "2px solid var(--destructive)", borderRadius: 8 } }
          : n,
      ),
    [nodes, errorIds],
  );

  // Panel open/close is click-driven. Canvas selection stays independent
  // so drag-select and keyboard delete keep working without opening panels.
  const openNodePanel = useCallback((_: React.MouseEvent, node: FlowNode) => {
    setOpenId(node.id);
  }, []);

  const closePanel = useCallback(() => {
    setOpenId(null);
  }, []);

  return (
    <div className="flex min-h-0 flex-1">
      <FlowPalette hideTriggers={ownerManaged} />
      <div
        className="relative min-w-0 flex-1 overflow-hidden bg-zinc-100 dark:bg-[#0b0b0e]"
        onDrop={onDrop}
        onDragOver={(e) => {
          e.preventDefault();
          e.dataTransfer.dropEffect = "move";
        }}
      >
        <ReactFlow
          nodes={displayNodes}
          edges={edges}
          onNodesChange={onNodesChangeSafe}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          isValidConnection={isValidConnection}
          onNodeClick={openNodePanel}
          onPaneClick={closePanel}
          nodeTypes={FLOW_NODE_TYPES}
          deleteKeyCode={["Backspace", "Delete"]}
          fitView
          defaultEdgeOptions={{ style: { stroke: "#71717a", strokeWidth: 2 } }}
          connectionLineStyle={{ stroke: "#a78bfa", strokeWidth: 2 }}
          minZoom={0.4}
          maxZoom={1.5}
        >
          <Background
            variant={BackgroundVariant.Dots}
            gap={18}
            size={1.5}
            color={dark ? "#3f3f46" : "#d4d4d8"}
            bgColor={dark ? "#0b0b0e" : "#f4f4f5"}
          />
          <Panel position="top-left" className="flex items-center gap-2">
            {actions}
            <Button size="sm" variant="outline" className="gap-1.5 border-border bg-popover hover:bg-muted dark:border-white/15 dark:bg-[#17171c] dark:text-zinc-100 dark:hover:bg-white/10 dark:hover:text-white" onClick={runValidation}>
              <ShieldCheck className="size-3.5" /> Validate
            </Button>
            <Button size="sm" className="gap-1.5" onClick={runTest}>
              <FlaskConical className="size-3.5" /> Test run
            </Button>
            <span className="ml-1 text-[11px] text-muted-foreground dark:text-zinc-500">
              {savedAt ? `Saved ${savedAt.toLocaleTimeString()}` : "Autosaves"}
            </span>
          </Panel>
          <Controls
            position="bottom-left"
            className="overflow-hidden rounded-lg border border-border bg-popover [&_button]:border-border [&_button]:bg-transparent [&_svg]:fill-zinc-500 dark:border-white/10 dark:bg-[#17171c] dark:[&_button]:border-white/10 dark:[&_svg]:fill-zinc-300"
          />
          <MiniMap
            position="bottom-right"
            pannable
            nodeColor={dark ? "#27272a" : "#e4e4e7"}
            maskColor={dark ? "rgba(0, 0, 0, 0.65)" : "rgba(255, 255, 255, 0.7)"}
            bgColor={dark ? "#0b0b0e" : "#f4f4f5"}
            className="rounded-lg border border-border dark:border-white/10"
          />
          {issues.length > 0 && (
            <Panel position="top-center">
              <div className="w-[min(28rem,90vw)] rounded-lg border border-border bg-popover p-3 text-popover-foreground shadow-xl">
                <div className="mb-2 flex items-center justify-between">
                  <p className="flex items-center gap-1.5 text-sm font-semibold">
                    <ListX className="size-4" />
                    {issues.filter((i) => i.severity === "error").length} problems
                  </p>
                  <Button variant="ghost" size="icon-xs" aria-label="Dismiss issues" onClick={() => setIssues([])}>
                    <X className="size-3.5" />
                  </Button>
                </div>
                <ul className="max-h-40 space-y-1.5 overflow-y-auto">
                  {issues.map((issue, i) => (
                    <li key={i} className="flex items-start gap-1.5 text-xs">
                      <CheckCircle2
                        className={cn(
                          "mt-0.5 size-3.5 shrink-0",
                          issue.severity === "error" ? "text-destructive" : "text-warning",
                        )}
                      />
                      <span className="text-muted-foreground">{issue.message}</span>
                    </li>
                  ))}
                </ul>
              </div>
            </Panel>
          )}
        </ReactFlow>

        {openNode && (
          <NodeConfigPanel
            key={openNode.id}
            node={openNode}
            onChange={updateNodeData}
            onClose={closePanel}
            lockedTrigger={ownerManaged}
          />
        )}
      </div>
    </div>
  );
}
