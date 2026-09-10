import { useRef, useState } from "react";
import RichTextEditor, { type RichTextEditorHandle } from "./RichTextEditor";
import { support } from "../../lib/api";
import type { AdminTicketMessage, AdminTicketStatus } from "../../lib/types";

function formatDate(iso: string) {
  return new Date(iso).toLocaleString();
}

function MessageBubble({ message }: { message: AdminTicketMessage }) {
  const isSupport = message.author === "support";
  return (
    <div
      className={`rounded-lg p-4 ${
        isSupport ? "bg-primary/5 border border-primary/10 ml-8" : "bg-muted/50 mr-8"
      }`}
    >
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-foreground">
            {message.authorName ?? (isSupport ? "Support" : "Customer")}
          </span>
          <span
            className={`text-xs px-1.5 py-0.5 rounded ${
              isSupport ? "bg-primary/10 text-primary" : "bg-secondary text-secondary-foreground"
            }`}
          >
            {message.author}
          </span>
        </div>
        <span className="text-xs text-muted-foreground">{formatDate(message.createdAt)}</span>
      </div>
      <div
        className="text-sm text-foreground prose prose-sm max-w-none"
        dangerouslySetInnerHTML={{ __html: message.body }}
      />
    </div>
  );
}

export default function TicketThread({
  ticketId,
  initialMessages,
  initialStatus,
}: {
  ticketId: string;
  initialMessages: AdminTicketMessage[];
  initialStatus: AdminTicketStatus;
}) {
  const [messages, setMessages] = useState(initialMessages);
  const [canSend, setCanSend] = useState(false);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const editorRef = useRef<RichTextEditorHandle>(null);
  const isClosed = initialStatus === "closed";

  async function refresh() {
    const res = await support.listMessages(ticketId);
    setMessages(res.messages);
  }

  async function send() {
    const html = editorRef.current?.getHTML() ?? "";
    if (!html || editorRef.current?.isEmpty()) return;
    setSending(true);
    setError(null);
    try {
      await support.sendReply(ticketId, html);
      editorRef.current?.clear();
      setCanSend(false);
      // A reply can move the ticket (resolved -> in_progress); reload so the
      // SSR header badge and composer state stay truthful.
      const ticket = await support.getTicket(ticketId);
      if (ticket.ticket.status !== initialStatus) {
        window.location.reload();
        return;
      }
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to send reply");
    } finally {
      setSending(false);
    }
  }

  return (
    <div className="space-y-4">
      {messages.length > 0 ? (
        messages.map((msg) => <MessageBubble key={msg.id} message={msg} />)
      ) : (
        <p className="text-sm text-muted-foreground text-center py-8">No messages yet.</p>
      )}

      <div className="border border-border rounded-lg p-4 mt-4">
        {isClosed ? (
          <p className="text-sm text-muted-foreground text-center py-4">
            This ticket is closed. Reopen it via the status control to reply.
          </p>
        ) : (
          <>
            <RichTextEditor
              ref={editorRef}
              placeholder="Type your response..."
              onUpdate={(_html, empty) => setCanSend(!empty)}
            />
            {error && <p className="text-xs text-destructive mt-2">{error}</p>}
            <div className="flex justify-end mt-3">
              <button
                type="button"
                onClick={send}
                disabled={!canSend || sending}
                className="inline-flex items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground shadow-sm transition-colors hover:bg-primary/80 disabled:opacity-50 disabled:pointer-events-none"
              >
                {sending ? "Sending..." : "Send Reply"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
