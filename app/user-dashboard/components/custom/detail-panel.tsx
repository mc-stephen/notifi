"use client";

import {
  useEffect,
  useSyncExternalStore,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";

/** Id of the dashboard body container (`app/(dashboard)/layout.tsx`) that hosts detail panels. */
export const DASHBOARD_PANEL_HOST_ID = "dashboard-panel-host";

function subscribeNoop() {
  return () => {};
}

type DetailPanelProps = {
  /** Dismiss handler — wired to Escape and the close button. */
  onClose: () => void;
  /** Header identity content (title, badges, subtitles). The shell adds the border + close button. */
  header: ReactNode;
  /** Body + footer markup. Bodies should start with a `flex-1 min-h-0 overflow-y-auto` scroller. */
  children: ReactNode;
  /** Width override. Defaults to `w-[min(24rem,92%)]`. */
  className?: string;
  /** Header padding override. Defaults to `px-5 py-4`. */
  headerClassName?: string;
  /** Accessible label for the close button. */
  closeLabel?: string;
};

/**
 * Shared right slide-over for row-click details (api keys, webhooks,
 * notifications, recipients, tickets, logs, provider settings).
 *
 * Portaled into the dashboard body container, so the panel always fills the
 * full height of the dashboard *page* — never just the table next to it —
 * and can never cover the sidebar or topbar (both live outside the host).
 * Every usage renders inside the dashboard layout, so the host always
 * exists by the time a panel opens (panels only open on click, post-mount).
 */
export function DetailPanel({
  onClose,
  header,
  children,
  className,
  headerClassName,
  closeLabel = "Close details",
}: DetailPanelProps) {
  // Mount guard for SSR: the host lookup needs `document`.
  const mounted = useSyncExternalStore(subscribeNoop, () => true, () => false);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const host = mounted
    ? document.getElementById(DASHBOARD_PANEL_HOST_ID)
    : null;
  if (!host) return null;

  return createPortal(
    <div className="absolute inset-0 z-20">
      {/* Frosted page backdrop — click anywhere outside the panel to dismiss. */}
      <div
        aria-hidden="true"
        onClick={onClose}
        className="absolute inset-0 bg-black/10 supports-backdrop-filter:backdrop-blur-xs animate-in fade-in duration-200"
      />
      <aside
        role="dialog"
        aria-label={closeLabel}
        className={cn(
          "absolute inset-y-0 right-0 flex min-h-0 w-[min(24rem,92%)] flex-col overflow-hidden border bg-popover text-card-foreground animate-in slide-in-from-right-3 fade-in duration-200",
          className,
        )}
      >
      <div
        className={cn(
          "flex items-start justify-between gap-2 border-b px-5 py-4",
          headerClassName,
        )}
      >
        <div className="min-w-0 flex-1">{header}</div>
        <button
          type="button"
          aria-label={closeLabel}
          className="rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
          onClick={onClose}
        >
          <X className="size-4" />
        </button>
      </div>
      {children}
      </aside>
    </div>,
    host,
  );
}
