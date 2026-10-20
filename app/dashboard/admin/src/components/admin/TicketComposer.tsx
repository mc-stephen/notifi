import { useRef, useState } from "react";
import RichTextEditor, { type RichTextEditorHandle } from "./RichTextEditor";
import { support } from "../../lib/api";
import type { AdminTicket } from "../../lib/types";

export default function TicketComposer({
  ticketId,
  ticketStatus,
  disabled,
  onSent,
}: {
  ticketId: string;
  ticketStatus: string;
  disabled: boolean;
  // Fired after a successful send (thread refreshes off this).
  onSent: (ticket: AdminTicket | null) => void;
}) {
  const [canSend, setCanSend] = useState(false);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const editorRef = useRef<RichTextEditorHandle>(null);

  async function send() {
    const html = editorRef.current?.getHTML() ?? "";
    if (!html || editorRef.current?.isEmpty() || sending) return;
    setSending(true);
    setError(null);
    try {
      await support.sendReply(ticketId, html);
      editorRef.current?.clear();
      setCanSend(false);
      // A reply can move the ticket (resolved -> in_progress); hand the
      // fresh ticket up so header, composer state, and list row resync.
      const ticket = await support.getTicket(ticketId);
      onSent(ticket.ticket.status !== ticketStatus ? ticket.ticket : null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to send reply");
    } finally {
      setSending(false);
    }
  }

  if (disabled) {
    return (
      <p className="text-sm text-muted-foreground text-center px-5 py-4">
        This ticket is closed. Reopen it via the status control to reply.
      </p>
    );
  }

  return (
    // Full-bleed composer: no x padding, flush with the drawer edges.
    <div className="border-t border-border py-3">
      <RichTextEditor
        ref={editorRef}
        placeholder="Type your response..."
        onUpdate={(_html, empty) => setCanSend(!empty)}
      />
      {error && <p className="text-xs text-destructive mt-2 px-5">{error}</p>}
      <div className="flex justify-end mt-3 px-5">
        <button
          type="button"
          onClick={send}
          disabled={!canSend || sending}
          className="inline-flex items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-sm transition-colors hover:bg-primary/80 disabled:opacity-50 disabled:pointer-events-none"
        >
          {sending ? "Sending..." : "Send Reply"}
        </button>
      </div>
    </div>
  );
}
