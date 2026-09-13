import { useEffect, useRef, useState, type RefObject } from "react";
import { support } from "../../lib/api";
import type { AdminTicketMessage, AdminTicketStatus } from "../../lib/types";

function formatTime(iso: string) {
  return new Date(iso).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

function fullDate(iso: string) {
  return new Date(iso).toLocaleString();
}

function formatDay(iso: string) {
  const d = new Date(iso);
  const today = new Date();
  const yesterday = new Date();
  yesterday.setDate(today.getDate() - 1);
  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate();
  if (sameDay(d, today)) return "Today";
  if (sameDay(d, yesterday)) return "Yesterday";
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

// Poll cadence for incoming customer replies (no realtime channel for
// one thread view; manual reload was the complaint).
const POLL_MS = 5000;

// Distance (px) from the bottom that still counts as "reading the latest".
const STICK_THRESHOLD_PX = 96;

type ChatEvent = {
  id: string;
  author: "customer" | "support";
  authorName: string;
  createdAt: string;
  bodyHtml?: string;
  bodyText?: string;
};

function ChatBubble({ event, firstOfGroup, lastOfGroup }: { event: ChatEvent; firstOfGroup: boolean; lastOfGroup: boolean }) {
  const isSupport = event.author === "support";
  return (
    <div className={`flex ${isSupport ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[78%] px-3 py-2 text-sm shadow-xs ${
          isSupport
            ? `bg-primary/15 text-foreground rounded-2xl ${firstOfGroup ? "rounded-tr-md" : ""}`
            : `bg-muted/70 text-foreground rounded-2xl ${firstOfGroup ? "rounded-tl-md" : ""}`
        }`}
      >
        {firstOfGroup && (
          <p className={`text-[11px] font-semibold mb-0.5 ${isSupport ? "text-primary" : "text-muted-foreground"}`}>
            {event.authorName}
          </p>
        )}
        {event.bodyHtml ? (
          <div
            className="prose prose-sm max-w-none [&_p]:my-0.5"
            dangerouslySetInnerHTML={{ __html: event.bodyHtml }}
          />
        ) : (
          <p className="whitespace-pre-wrap">{event.bodyText}</p>
        )}
        {lastOfGroup && (
          <p
            title={fullDate(event.createdAt)}
            className={`float-right ml-2 mt-1 text-[10px] leading-none ${
              isSupport ? "text-foreground/60" : "text-muted-foreground"
            }`}
          >
            {formatTime(event.createdAt)}
          </p>
        )}
      </div>
    </div>
  );
}

export type TicketOpener = {
  body: string;
  authorName: string;
  createdAt: string;
};

export default function TicketThread({
  ticketId,
  opener,
  initialMessages,
  initialStatus,
  sendSignal,
  scrollContainerRef,
}: {
  ticketId: string;
  opener: TicketOpener;
  initialMessages: AdminTicketMessage[];
  initialStatus: AdminTicketStatus;
  // Bumped by the footer composer after each send: refresh the thread.
  sendSignal: number;
  // The scrollable ancestor (owned by the drawer): smart autoscroll and
  // the new-messages pill attach to it.
  scrollContainerRef?: RefObject<HTMLDivElement | null>;
}) {
  const [messages, setMessages] = useState(initialMessages);
  // True while the view sits at the latest messages; cleared when the
  // reader scrolls up, and with it the auto-stick.
  const [stuckToBottom, setStuckToBottom] = useState(true);
  // Visible "↓ New messages" pill after arrivals while scrolled up.
  const [hasNewBelow, setHasNewBelow] = useState(false);
  const messageCountRef = useRef(initialMessages.length);
  const isClosed = initialStatus === "closed";

  async function refresh() {
    const res = await support.listMessages(ticketId);
    setMessages(res.messages);
  }

  function scrollToBottom() {
    const el = scrollContainerRef?.current;
    if (el) el.scrollTop = el.scrollHeight;
    setStuckToBottom(true);
    setHasNewBelow(false);
  }

  // Track whether the reader sits at the latest messages.
  useEffect(() => {
    const el = scrollContainerRef?.current;
    if (!el) return;
    const onScroll = () => {
      const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < STICK_THRESHOLD_PX;
      setStuckToBottom(nearBottom);
      if (nearBottom) setHasNewBelow(false);
    };
    onScroll();
    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, [scrollContainerRef, ticketId]);

  // Stick to the bottom on arrivals only while reading the latest;
  // otherwise raise the pill.
  const stuckRef = useRef(stuckToBottom);
  stuckRef.current = stuckToBottom;
  useEffect(() => {
    if (messages.length <= messageCountRef.current) {
      messageCountRef.current = messages.length;
      return;
    }
    messageCountRef.current = messages.length;
    if (stuckRef.current) {
      const el = scrollContainerRef?.current;
      if (el) el.scrollTop = el.scrollHeight;
    } else {
      setHasNewBelow(true);
    }
  }, [messages, scrollContainerRef]);

  // Open at the latest messages.
  useEffect(() => {
    const el = scrollContainerRef?.current;
    if (el) el.scrollTop = el.scrollHeight;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ticketId]);

  // Live thread: pick up customer replies without a full-page reload.
  // Skipped while hidden or on closed tickets. Poll reads are idempotent,
  // so no send-guard is needed (composer lives outside the thread now).
  useEffect(() => {
    if (isClosed) return;
    const timer = window.setInterval(async () => {
      if (document.hidden) return;
      try {
        await refresh();
      } catch {
        // Next tick retries; transient failures stay silent.
      }
    }, POLL_MS);
    return () => window.clearInterval(timer);
  }, [ticketId, isClosed]);

  // Footer composer signals a completed send: refresh the thread.
  const signalRef = useRef(sendSignal);
  useEffect(() => {
    if (sendSignal === signalRef.current) return;
    signalRef.current = sendSignal;
    refresh().catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sendSignal]);

  // Chronological events, clustered into same-author runs.
  const events: ChatEvent[] = [
    {
      id: "opener",
      author: "customer",
      authorName: opener.authorName,
      createdAt: opener.createdAt,
      bodyText: opener.body,
    },
    ...messages.map((msg) => ({
      id: msg.id,
      author: msg.author,
      authorName: msg.authorName ?? (msg.author === "support" ? "Support" : "Customer"),
      createdAt: msg.createdAt,
      bodyHtml: msg.body,
    })),
  ];
  const groups: ChatEvent[][] = [];
  for (const event of events) {
    const run = groups[groups.length - 1];
    if (run && run[0].author === event.author) run.push(event);
    else groups.push([event]);
  }

  let lastDay = "";

  return (
    <div className="space-y-4">
      {groups.map((group) => {
        const day = formatDay(group[0].createdAt);
        const showDay = day !== lastDay;
        lastDay = day;
        return (
          <div key={group[0].id}>
            {showDay && (
              <div className="flex justify-center pb-3">
                <span className="rounded-md bg-muted px-2.5 py-1 text-[11px] font-medium text-muted-foreground">
                  {day}
                </span>
              </div>
            )}
            <div className="space-y-[3px]">
              {group.map((event, i) => (
                <ChatBubble
                  key={event.id}
                  event={event}
                  firstOfGroup={i === 0}
                  lastOfGroup={i === group.length - 1}
                />
              ))}
            </div>
          </div>
        );
      })}

      {hasNewBelow && (
        <div className="sticky bottom-4 flex justify-center">
          <button
            type="button"
            onClick={scrollToBottom}
            className="inline-flex items-center gap-1.5 rounded-full bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground shadow-lg transition-colors hover:bg-primary/90"
          >
            ↓ New messages
          </button>
        </div>
      )}
    </div>
  );
}
