import { useState } from "react";
import { support } from "../../lib/api";
import { TICKET_STATUSES, TICKET_STATUS_LABELS, type TicketStatus } from "../../lib/constants";

export default function TicketStatusSelect({
  ticketId,
  initialStatus,
}: {
  ticketId: string;
  initialStatus: TicketStatus;
}) {
  const [status, setStatus] = useState<TicketStatus>(initialStatus);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function change(next: TicketStatus) {
    if (next === status || saving) return;
    setSaving(true);
    setError(null);
    try {
      const res = await support.setStatus(ticketId, next);
      setStatus(res.ticket.status);
      // Status drives SSR markup (header badge, composer state) — reload.
      window.location.reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to update status");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-2">
      <label className="text-xs font-medium text-muted-foreground" htmlFor="ticket-status">
        Status
      </label>
      <select
        id="ticket-status"
        value={status}
        disabled={saving}
        onChange={(e) => change(e.target.value as TicketStatus)}
        className="flex h-9 w-full rounded-lg border border-input bg-transparent px-3 py-1.5 text-sm shadow-xs focus-visible:border-ring focus-visible:outline-none disabled:opacity-50"
      >
        {TICKET_STATUSES.map((s) => (
          <option key={s} value={s}>
            {TICKET_STATUS_LABELS[s]}
          </option>
        ))}
      </select>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
