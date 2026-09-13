import { useEffect, useRef, useState } from "react";
import type { ColumnDef } from "@tanstack/react-table";
import { DataTable } from "../ui/data-table";
import { support } from "../../lib/api";
import { OPEN_TICKET_EVENT, TICKET_UPDATED_EVENT } from "./TicketDrawerHost";
import { TICKET_STATUSES, TICKET_STATUS_COLORS, TICKET_STATUS_LABELS, PRIORITY_COLORS } from "../../lib/constants";
import type { AdminTicket } from "../../lib/types";

const LIMIT = 25;
const SEARCH_DEBOUNCE_MS = 350;

function priorityColor(priority: string) {
  return PRIORITY_COLORS[priority.toLowerCase()] ?? "bg-muted text-muted-foreground border-border";
}

function openTicket(id: string) {
  window.dispatchEvent(new CustomEvent<string>(OPEN_TICKET_EVENT, { detail: id }));
}

const COLUMNS: ColumnDef<AdminTicket, unknown>[] = [
  {
    accessorKey: "subject",
    header: "Ticket",
    cell: ({ row }) => (
      <span>
        <a
          href={`/support?ticket=${encodeURIComponent(row.original.id)}`}
          onClick={(e) => {
            // Modifier/middle clicks follow the real link (shareable URL
            // auto-opens the drawer on load).
            if (e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return;
            e.preventDefault();
            openTicket(row.original.id);
          }}
          className="font-medium text-foreground hover:underline block"
        >
          {row.original.subject}
        </a>
        <p className="text-xs text-muted-foreground mt-0.5">{row.original.id}</p>
      </span>
    ),
  },
  {
    accessorKey: "customerName",
    header: "Customer",
    cell: ({ row }) => (
      <span>
        <p className="text-foreground">{row.original.customerName}</p>
        <p className="text-xs text-muted-foreground">{row.original.customerEmail}</p>
      </span>
    ),
  },
  {
    accessorKey: "status",
    header: "Status",
    cell: ({ row }) => (
      <span
        className={`inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium ${TICKET_STATUS_COLORS[row.original.status]}`}
      >
        {TICKET_STATUS_LABELS[row.original.status]}
      </span>
    ),
  },
  {
    accessorKey: "priority",
    header: "Priority",
    cell: ({ row }) => (
      <span
        className={`inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium capitalize ${priorityColor(row.original.priority)}`}
      >
        {row.original.priority}
      </span>
    ),
  },
  {
    accessorKey: "updatedAt",
    header: "Updated",
    cell: ({ row }) => (
      <span className="text-xs text-muted-foreground">
        {new Date(row.original.updatedAt).toLocaleDateString()}
      </span>
    ),
  },
];

export default function TicketsTable({
  initialTickets,
  initialTotal,
  initialPage,
  initialStatus,
  initialQuery,
}: {
  initialTickets: AdminTicket[];
  initialTotal: number;
  initialPage: number;
  initialStatus: string;
  initialQuery: string;
}) {
  const [tickets, setTickets] = useState(initialTickets);
  const [total, setTotal] = useState(initialTotal);
  const [totalPages, setTotalPages] = useState(() => Math.max(1, Math.ceil(initialTotal / LIMIT)));
  const [page, setPage] = useState(initialPage);
  const [pageSize, setPageSize] = useState(LIMIT);
  const [status, setStatus] = useState(initialStatus);
  const [query, setQuery] = useState(initialQuery);
  const [debouncedQuery, setDebouncedQuery] = useState(initialQuery);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const firstLoad = useRef(true);

  // Debounce the search box into queries.
  useEffect(() => {
    const timer = window.setTimeout(() => setDebouncedQuery(query), SEARCH_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [query]);

  // Refetch on filter/page change (skipped for the SSR first paint).
  useEffect(() => {
    if (firstLoad.current) {
      firstLoad.current = false;
      return;
    }
    let ignore = false;
    (async () => {
      setLoading(true);
      setError(null);
      try {
        const res = await support.listTickets({
          status: status || undefined,
          q: debouncedQuery || undefined,
          limit: pageSize,
          page,
        });
        if (ignore) return;
        setTickets(res.tickets);
        setTotal(res.total);
        setTotalPages(Math.max(1, res.totalPages));
      } catch {
        if (ignore) return;
        setError("Could not load tickets. Check that the API is running and try again.");
      } finally {
        if (!ignore) setLoading(false);
      }
    })();
    return () => {
      ignore = true;
    };
  }, [page, pageSize, status, debouncedQuery]);

  // Drawer edits patch rows in place — no list reload.
  useEffect(() => {
    const onUpdated = (e: Event) => {
      const next = (e as CustomEvent<AdminTicket>).detail;
      setTickets((prev) => prev.map((t) => (t.id === next.id ? next : t)));
    };
    window.addEventListener(TICKET_UPDATED_EVENT, onUpdated);
    return () => window.removeEventListener(TICKET_UPDATED_EVENT, onUpdated);
  }, []);

  return (
    <div className={loading ? "opacity-60 pointer-events-none" : undefined}>
      {error && (
        <div className="rounded-xl border border-destructive/30 bg-destructive/5 px-4 py-3 mb-3">
          <p className="text-sm text-destructive">{error}</p>
        </div>
      )}
      <DataTable
        columns={COLUMNS}
        data={tickets}
        searchPlaceholder="Search tickets..."
        searchValue={query}
        onSearchChange={(v) => {
          setQuery(v);
          setPage(1);
        }}
        emptyMessage="No tickets found. Try adjusting your search or filters."
        onRowClick={(row) => openTicket(row.id)}
        tableClassName="bg-card"
        manualPagination
        pageCount={totalPages}
        pageIndex={page - 1}
        pageSize={pageSize}
        onPageChange={(i) => setPage(i + 1)}
        onPageSizeChange={(s) => {
          setPageSize(s);
          setPage(1);
        }}
        totalCount={total}
        renderToolbar={() => (
          <div className="flex items-center gap-3">
            <div className="relative max-w-sm flex-1">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
              <input
                type="text"
                value={query}
                onChange={(e) => {
                  setQuery(e.target.value);
                  setPage(1);
                }}
                placeholder="Search tickets..."
                aria-label="Search tickets"
                className="flex h-8 w-full rounded-lg border border-input bg-card pl-8 pr-3 py-1.5 text-sm transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:outline-none"
              />
            </div>
            <select
              value={status}
              onChange={(e) => {
                setStatus(e.target.value);
                setPage(1);
              }}
              aria-label="Filter by status"
              className="flex h-8 rounded-lg border border-input bg-card px-3 py-1.5 text-sm focus-visible:border-ring focus-visible:outline-none"
            >
              <option value="">All Status</option>
              {TICKET_STATUSES.map((s) => (
                <option key={s} value={s}>
                  {TICKET_STATUS_LABELS[s]}
                </option>
              ))}
            </select>
          </div>
        )}
      />
    </div>
  );
}
