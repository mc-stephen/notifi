import { useState } from "react";
import { support } from "../../lib/api";
import { TICKET_PRIORITIES, TICKET_PRIORITY_LABELS, type TicketPriority } from "../../lib/constants";

export default function TicketPrioritySelect({
  ticketId,
  initialPriority,
  onChanged,
}: {
  ticketId: string;
  initialPriority: string;
  // When provided, the parent syncs state itself (drawer) instead of
  // reloading the page.
  onChanged?: (priority: string) => void;
}) {
  const [priority, setPriority] = useState(initialPriority);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function change(next: string) {
    if (next === priority || saving) return;
    setSaving(true);
    setError(null);
    try {
      const res = await support.setPriority(ticketId, next as TicketPriority);
      setPriority(res.ticket.priority);
      if (onChanged) {
        onChanged(res.ticket.priority);
        return;
      }
      // Priority drives SSR markup (header badge) — reload.
      window.location.reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to update priority");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-2">
      <label className="text-xs font-medium text-muted-foreground" htmlFor="ticket-priority">
        Priority
      </label>
      <select
        id="ticket-priority"
        value={TICKET_PRIORITIES.includes(priority as TicketPriority) ? priority : "medium"}
        disabled={saving}
        onChange={(e) => change(e.target.value)}
        className="flex h-9 w-full rounded-lg border border-input bg-transparent px-3 py-1.5 text-sm shadow-xs focus-visible:border-ring focus-visible:outline-none disabled:opacity-50 capitalize"
      >
        {TICKET_PRIORITIES.map((p) => (
          <option key={p} value={p}>
            {TICKET_PRIORITY_LABELS[p]}
          </option>
        ))}
      </select>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
