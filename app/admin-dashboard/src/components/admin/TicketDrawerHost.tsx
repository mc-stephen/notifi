import { useCallback, useEffect, useRef, useState } from "react";
import TicketThread from "./TicketThread";
import TicketComposer from "./TicketComposer";
import TicketStatusSelect from "./TicketStatusSelect";
import TicketPrioritySelect from "./TicketPrioritySelect";
import { support } from "../../lib/api";
import { TICKET_STATUS_COLORS, TICKET_STATUS_LABELS, PRIORITY_COLORS } from "../../lib/constants";
import type { AdminTicket, AdminTicketMessage } from "../../lib/types";

export const OPEN_TICKET_EVENT = "open-ticket";
export const TICKET_UPDATED_EVENT = "ticket-updated";

function initials(name: string) {
  return name
    .split(" ")
    .map((n) => n[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
}

function relativeTime(iso: string) {
  const seconds = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return new Date(iso).toLocaleDateString();
}

function priorityColor(priority: string) {
  return PRIORITY_COLORS[priority.toLowerCase()] ?? "bg-muted text-muted-foreground border-border";
}

function ticketIdFromUrl() {
  return new URLSearchParams(window.location.search).get("ticket");
}

function setTicketParam(id: string | null) {
  const url = new URL(window.location.href);
  if (id) url.searchParams.set("ticket", id);
  else url.searchParams.delete("ticket");
  window.history.replaceState(null, "", url.toString());
}

export default function TicketDrawerHost() {
  const [openId, setOpenId] = useState<string | null>(null);
  const [ticket, setTicket] = useState<AdminTicket | null>(null);
  const [messages, setMessages] = useState<AdminTicketMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  // Bumped by the footer composer after each send: the thread refreshes.
  const [sendSignal, setSendSignal] = useState(0);
  const scrollRef = useRef<HTMLDivElement | null>(null);

  const close = useCallback(() => {
    setOpenId(null);
    setTicket(null);
    setMessages([]);
    setLoadError(null);
    setTicketParam(null);
  }, []);

  const open = useCallback(async (id: string) => {
    setOpenId(id);
    setTicket(null);
    setMessages([]);
    setLoadError(null);
    setLoading(true);
    setTicketParam(id);
    try {
      const [ticketRes, messagesRes] = await Promise.all([
        support.getTicket(id),
        support.listMessages(id),
      ]);
      setTicket(ticketRes.ticket);
      setMessages(messagesRes.messages ?? []);
    } catch {
      setLoadError("Could not load this ticket. Check that the API is running and try again.");
    } finally {
      setLoading(false);
    }
  }, []);

  // Row clicks + shared ?ticket= links funnel through here.
  useEffect(() => {
    const onOpen = (e: Event) => open((e as CustomEvent<string>).detail);
    window.addEventListener(OPEN_TICKET_EVENT, onOpen);
    const deep = ticketIdFromUrl();
    if (deep) void open(deep);
    return () => window.removeEventListener(OPEN_TICKET_EVENT, onOpen);
  }, [open ]);

  // Escape closes. No scroll lock: the drawer is page-anchored, so
  // nothing outside the page needs freezing.
  useEffect(() => {
    if (!openId) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [openId, close]);

  function syncTicket(next: AdminTicket) {
    setTicket(next);
    window.dispatchEvent(new CustomEvent(TICKET_UPDATED_EVENT, { detail: next }));
  }

  function handleSent(moved: AdminTicket | null) {
    if (moved) syncTicket(moved);
    setSendSignal((s) => s + 1);
  }

  if (!openId) return null;

  return (
    // Viewport-locked below the topbar: full height without participating
    // in the page's scroll math (an oversized absolute box manufactures
    // phantom scroll space). Sidebar/topbar stay visible and undimmed.
    <div
      className="fixed inset-x-0 top-14 bottom-0 z-20 lg:left-64"
      role="dialog"
      aria-modal="true"
      aria-label="Ticket details"
    >
      <div className="absolute inset-0 bg-black/60" onClick={close} aria-hidden />
      <div className="absolute top-0 right-0 bottom-0 flex w-[min(44rem,94%)] flex-col border-l border-border bg-background shadow-2xl animate-in slide-in-from-right-3 fade-in duration-200">
        {loading || !ticket ? (
          <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
            {loadError ? (
              <div className="text-center space-y-3">
                <p>{loadError}</p>
                <button
                  type="button"
                  onClick={close}
                  className="inline-flex items-center justify-center rounded-lg border border-input bg-background px-3 py-2 text-sm font-medium hover:bg-muted transition-colors"
                >
                  Close
                </button>
              </div>
            ) : (
              "Loading ticket…"
            )}
          </div>
        ) : (
          <>
            <div className="border-b border-border px-5 py-4">
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center gap-2">
                    <h2 className="text-lg font-bold tracking-tight text-foreground truncate">
                      {ticket.subject}
                    </h2>
                    <span className="text-[11px] text-muted-foreground font-mono">
                      {ticket.id.slice(0, 8)}
                    </span>
                  </div>
                  <div className="mt-2 flex flex-wrap items-center gap-2">
                    <span
                      className={`inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium ${TICKET_STATUS_COLORS[ticket.status]}`}
                    >
                      {TICKET_STATUS_LABELS[ticket.status]}
                    </span>
                    <span
                      className={`inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium capitalize ${priorityColor(ticket.priority)}`}
                    >
                      {ticket.priority}
                    </span>
                  </div>
                </div>
                <button
                  type="button"
                  onClick={close}
                  aria-label="Close ticket"
                  className="rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground shrink-0"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
                </button>
              </div>
              <div className="mt-3 flex items-center gap-2.5">
                <div className="flex size-8 items-center justify-center rounded-full bg-muted text-[11px] font-semibold text-muted-foreground">
                  {initials(ticket.customerName)}
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-medium text-foreground truncate">{ticket.customerName}</p>
                  <p className="text-xs text-muted-foreground truncate">
                    {ticket.customerEmail} · opened {relativeTime(ticket.createdAt)}
                    {ticket.projectId ? " · project" : " · personal"}
                  </p>
                </div>
              </div>
            </div>

            <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
              <TicketThread
                key={`${ticket.id}:${ticket.status}`}
                ticketId={ticket.id}
                opener={{
                  body: ticket.description,
                  authorName: ticket.customerName,
                  createdAt: ticket.createdAt,
                }}
                initialMessages={messages}
                initialStatus={ticket.status}
                sendSignal={sendSignal}
                scrollContainerRef={scrollRef}
              />
            </div>

            <div className="border-t border-border">
              <div className="grid grid-cols-2 gap-3 px-5 py-3">
                <TicketStatusSelect
                  ticketId={ticket.id}
                  initialStatus={ticket.status}
                  onChanged={(status) =>
                    ticket && syncTicket({ ...ticket, status })
                  }
                />
                <TicketPrioritySelect
                  ticketId={ticket.id}
                  initialPriority={ticket.priority}
                  onChanged={(priority) =>
                    ticket && syncTicket({ ...ticket, priority })
                  }
                />
              </div>
              <TicketComposer
                ticketId={ticket.id}
                ticketStatus={ticket.status}
                disabled={ticket.status === "closed"}
                onSent={handleSent}
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
