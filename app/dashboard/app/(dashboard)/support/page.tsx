"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import { type ColumnDef } from "@tanstack/react-table";
import { toast } from "sonner";
import { PageHeader } from "@/components/custom/page-header";
import { DataTable } from "@/components/custom/data-table";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { env, links } from "@/lib/env";
import { useProjectStore } from "@/store/project-store";
import {
  useSupportTickets,
  useSubmitTicket,
  useTicketThread,
  useSendReply,
} from "@/hooks/use-support";
import type { SupportTicket, TicketStatus } from "@/lib/types";
import { format } from "date-fns";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Search,
  BookOpen,
  MessageSquare,
  ExternalLink,
  FileText,
  Zap,
  Mail,
  Clock,
  CheckCircle2,
  Loader2,
  X,
  Send,
  User,
  Headphones,
  Paperclip,
} from "lucide-react";

const STATUS_STYLES: Record<TicketStatus, string> = {
  open: "bg-info/10 text-info border-info/20",
  in_progress: "bg-warning/10 text-warning border-warning/20",
  resolved: "bg-success/10 text-success border-success/20",
  closed: "bg-muted text-muted-foreground border-border",
};

const STATUS_LABELS: Record<TicketStatus, string> = {
  open: "Open",
  in_progress: "In Progress",
  resolved: "Resolved",
  closed: "Closed",
};

export default function SupportPage() {
  const { tickets, loading: ticketsLoading, error: ticketsError, refresh } = useSupportTickets();
  const { submit } = useSubmitTicket();
  const currentProject = useProjectStore((s) => s.currentProject);

  const [faqSearch, setFaqSearch] = useState("");
  const [ticketDialog, setTicketDialog] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  // Right-side detail panel selection.
  const [details, setDetails] = useState<SupportTicket | null>(null);

  // Form state
  const [formSubject, setFormSubject] = useState("");
  const [formCategory, setFormCategory] = useState("Technical Issue");
  const [formPriority, setFormPriority] = useState("Medium - Issue affecting workflow");
  const [formDescription, setFormDescription] = useState("");
  const [formScope, setFormScope] = useState<"project" | "personal">(
    currentProject ? "project" : "personal",
  );

  const resetForm = useCallback(() => {
    setFormSubject("");
    setFormCategory("Technical Issue");
    setFormPriority("Medium - Issue affecting workflow");
    setFormDescription("");
    setFormScope(currentProject ? "project" : "personal");
  }, [currentProject]);

  const handleSubmit = useCallback(async () => {
    setSubmitting(true);
    try {
      await submit({
        projectId: formScope === "project" ? currentProject?.id : null,
        subject: formSubject,
        category: formCategory,
        priority: formPriority.split(" - ")[0],
        description: formDescription,
      });
      toast.success("Ticket submitted successfully");
      setTicketDialog(false);
      resetForm();
      refresh();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to submit ticket");
    } finally {
      setSubmitting(false);
    }
  }, [submit, formScope, formSubject, formCategory, formPriority, formDescription, currentProject, resetForm, refresh]);

  // FAQs
  const faqs = [
    { question: "How do I get my API key?", answer: "Navigate to API Keys in your dashboard and click 'Generate key'. Choose the appropriate environment (Production, Staging, or Development).", category: "Getting Started" },
    { question: "How do I send my first notification?", answer: "Install the SDK for your language, initialize the client with your API key, and call sendNotification() with a recipient ID and message body.", category: "Getting Started" },
    { question: "What are the rate limits?", answer: "Rate limits depend on your plan. Free: 100 req/min, Starter: 500 req/min, Pro: 1000 req/min, Enterprise: Custom.", category: "Limits" },
    { question: "How do I add a new notification channel?", answer: "Go to Channels page, select the channel type you want to configure, and follow the setup wizard with your provider credentials.", category: "Channels" },
    { question: "How do webhooks work?", answer: "Webhooks deliver real-time event notifications to your endpoint. Configure the URL, select events, and we'll POST signed payloads to your server.", category: "Webhooks" },
    { question: "Can I test notifications before going live?", answer: "Yes, use the Staging or Development environment keys to send test notifications without affecting production metrics.", category: "Getting Started" },
  ];

  const filteredFaqs = faqSearch
    ? faqs.filter((f) =>
        f.question.toLowerCase().includes(faqSearch.toLowerCase()) ||
        f.answer.toLowerCase().includes(faqSearch.toLowerCase())
      )
    : faqs;

  // Table columns
  const columns: ColumnDef<SupportTicket, unknown>[] = [
    {
      accessorKey: "subject",
      header: "Subject",
      cell: ({ row }) => (
        <span className="text-sm font-medium">{row.original.subject}</span>
      ),
    },
    {
      accessorKey: "category",
      header: "Category",
      cell: ({ row }) => (
        <Badge variant="secondary" className="text-[10px]">{row.original.category}</Badge>
      ),
    },
    {
      accessorKey: "priority",
      header: "Priority",
      cell: ({ row }) => (
        <span className="text-xs">{row.original.priority}</span>
      ),
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ row }) => (
        <span className={`inline-flex items-center rounded-md border px-1.5 py-0.5 text-[10px] font-medium ${STATUS_STYLES[row.original.status]}`}>
          {STATUS_LABELS[row.original.status]}
        </span>
      ),
    },
    {
      id: "scope",
      header: "Scope",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground">
          {row.original.projectId ? "Project" : "Personal"}
        </span>
      ),
    },
    {
      accessorKey: "createdAt",
      header: "Created",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {format(new Date(row.original.createdAt), "MMM d, HH:mm")}
        </span>
      ),
    },
  ];

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-6">
      <PageHeader
        title="Support & Help"
        description="Get help with Notifi"
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Support" }]}
        actions={
          <Button size="sm" className="gap-1.5" onClick={() => setTicketDialog(true)}>
            <Mail className="size-3.5" /> Submit ticket
          </Button>
        }
      />

      <Tabs defaultValue="tickets" className="min-h-0 flex-1">
        <TabsList className="shrink-0">
          <TabsTrigger value="tickets">My Tickets</TabsTrigger>
          <TabsTrigger value="help">Help & FAQs</TabsTrigger>
        </TabsList>

        {/* Tickets tab */}
        <TabsContent value="tickets" className="mt-4 flex min-h-0 flex-col">
          <div className="flex min-h-0 flex-1 items-stretch gap-6">
            <div className="min-w-0 flex-1 overflow-y-auto">
              {ticketsLoading ? (
                <div className="overflow-hidden rounded-lg border bg-card">
                  <div className="flex items-center justify-center py-12 text-muted-foreground">
                    <Loader2 className="size-5 animate-spin mr-2" /> Loading tickets…
                  </div>
                </div>
              ) : ticketsError ? (
                <div className="overflow-hidden rounded-lg border bg-card">
                  <div className="flex items-center justify-center py-12 text-center text-muted-foreground">
                    {ticketsError}
                  </div>
                </div>
              ) : (
                <DataTable
                  columns={columns}
                  data={tickets}
                  searchKey="subject"
                  searchPlaceholder="Search tickets..."
                  emptyMessage="No tickets yet. Submit one to get started."
                  pageSize={10}
                  tableClassName="bg-card"
                  onRowClick={(row) => setDetails(row)}
                />
              )}
            </div>

            {details && (
              <TicketDetailPanel
                key={details.id}
                ticket={details}
                onClose={() => setDetails(null)}
                onTicketUpdate={(updated) => {
                  setDetails(updated);
                  refresh();
                }}
              />
            )}
          </div>
        </TabsContent>

        {/* Help tab */}
        <TabsContent value="help" className="mt-4 overflow-y-auto">
          <div className="grid gap-4 md:grid-cols-3 mb-6">
            <Card>
              <CardContent className="pt-5">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">Documentation</span>
                  <BookOpen className="size-4 text-muted-foreground" />
                </div>
                <a href={links.docs} target="_blank" rel="noopener noreferrer" className="text-sm text-primary hover:underline mt-1 block">
                  Browse docs →
                </a>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="pt-5">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">API Reference</span>
                  <Zap className="size-4 text-muted-foreground" />
                </div>
                <a href={links.docsApi} target="_blank" rel="noopener noreferrer" className="text-sm text-primary hover:underline mt-1 block">
                  View API docs →
                </a>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="pt-5">
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">Response Time</span>
                  <Clock className="size-4 text-muted-foreground" />
                </div>
                <div className="text-2xl font-bold mt-1">&lt; 4h</div>
                <p className="text-xs text-muted-foreground mt-1">Average ticket response</p>
              </CardContent>
            </Card>
          </div>

          <div className="grid gap-6 lg:grid-cols-[1fr_320px]">
            <div className="space-y-4">
              <h2 className="text-lg font-semibold">Frequently Asked Questions</h2>
              <div className="relative max-w-sm">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
                <Input
                  placeholder="Search FAQs..."
                  value={faqSearch}
                  onChange={(e) => setFaqSearch(e.target.value)}
                  className="pl-8 h-8 text-sm"
                />
              </div>
              <div className="space-y-3">
                {filteredFaqs.map((faq, i) => (
                  <Card key={i}>
                    <CardContent className="pt-4 pb-4 space-y-2">
                      <div className="flex items-start justify-between">
                        <h3 className="text-sm font-medium">{faq.question}</h3>
                        <Badge variant="secondary" className="text-[10px] shrink-0 ml-2">{faq.category}</Badge>
                      </div>
                      <p className="text-xs text-muted-foreground">{faq.answer}</p>
                    </CardContent>
                  </Card>
                ))}
                {filteredFaqs.length === 0 && (
                  <p className="text-sm text-muted-foreground text-center py-8">No matching questions found.</p>
                )}
              </div>
            </div>

            <div className="space-y-4">
              <h2 className="text-lg font-semibold">Resources</h2>
              <Card>
                <CardHeader>
                  <CardTitle className="text-sm">Quick Links</CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  {[
                    { label: "Getting Started Guide", href: links.docsGettingStarted, icon: BookOpen },
                    { label: "API Reference", href: links.docsApi, icon: Zap },
                    { label: "SDK Documentation", href: links.docsSdks, icon: FileText },
                    { label: "Changelog", href: links.docsChangelog, icon: FileText },
                    { label: "Status Page", href: env.status, icon: CheckCircle2 },
                  ].map((link) => (
                    <a
                      key={link.label}
                      href={link.href}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
                    >
                      <link.icon className="size-3.5" />
                      <span>{link.label}</span>
                      <ExternalLink className="size-2.5 ml-auto" />
                    </a>
                  ))}
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="text-sm">Contact Support</CardTitle>
                </CardHeader>
                <CardContent className="space-y-3">
                  <p className="text-xs text-muted-foreground">
                    For account-specific issues, submit a support ticket. Our team responds within 4 hours for Pro plans.
                  </p>
                  <Button size="sm" className="w-full gap-1.5" onClick={() => setTicketDialog(true)}>
                    <MessageSquare className="size-3.5" /> Submit ticket
                  </Button>
                </CardContent>
              </Card>
            </div>
          </div>
        </TabsContent>
      </Tabs>

      {/* Submit ticket dialog */}
      <Dialog open={ticketDialog} onOpenChange={setTicketDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Submit support ticket</DialogTitle>
            <DialogDescription>Describe your issue and our team will get back to you.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label>Scope</Label>
              <div className="flex gap-2">
                <Button
                  variant={formScope === "project" ? "default" : "outline"}
                  size="sm"
                  onClick={() => setFormScope("project")}
                  disabled={!currentProject}
                >
                  This project{currentProject ? ` (${currentProject.name})` : ""}
                </Button>
                <Button
                  variant={formScope === "personal" ? "default" : "outline"}
                  size="sm"
                  onClick={() => setFormScope("personal")}
                >
                  Personal / Account
                </Button>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="ticket-subject">Subject</Label>
              <Input
                id="ticket-subject"
                placeholder="Brief description of the issue"
                value={formSubject}
                onChange={(e) => setFormSubject(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label>Category</Label>
              <select
                className="w-full rounded-md border bg-transparent px-3 py-2 text-sm"
                value={formCategory}
                onChange={(e) => setFormCategory(e.target.value)}
              >
                <option>Technical Issue</option>
                <option>Billing</option>
                <option>Account Access</option>
                <option>Feature Request</option>
                <option>Integration Help</option>
                <option>Other</option>
              </select>
            </div>
            <div className="space-y-2">
              <Label>Priority</Label>
              <select
                className="w-full rounded-md border bg-transparent px-3 py-2 text-sm"
                value={formPriority}
                onChange={(e) => setFormPriority(e.target.value)}
              >
                <option>Low - General question</option>
                <option>Medium - Issue affecting workflow</option>
                <option>High - Service disruption</option>
                <option>Urgent - Production down</option>
              </select>
            </div>
            <div className="space-y-2">
              <Label>Message</Label>
              <textarea
                placeholder="Provide details about your issue, including steps to reproduce, expected vs actual behavior, and any error messages..."
                className="w-full min-h-[150px] rounded-md border bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring resize-y"
                value={formDescription}
                onChange={(e) => setFormDescription(e.target.value)}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setTicketDialog(false)}>
              Cancel
            </Button>
            <Button
              onClick={handleSubmit}
              disabled={submitting || !formSubject.trim() || !formDescription.trim()}
            >
              {submitting ? "Submitting…" : "Submit ticket"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function TicketDetailPanel({
  ticket,
  onClose,
  onTicketUpdate,
}: {
  ticket: SupportTicket;
  onClose: () => void;
  onTicketUpdate: (ticket: SupportTicket) => void;
}) {
  const { messages, loading, error, refresh } = useTicketThread(ticket.id);
  const { sendReply } = useSendReply();
  const [replyBody, setReplyBody] = useState("");
  const [sending, setSending] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const isClosed = ticket.status === "closed";

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [messages]);

  const handleSendReply = useCallback(async () => {
    if (!replyBody.trim() || sending) return;
    setSending(true);
    try {
      await sendReply(ticket.id, replyBody.trim());
      setReplyBody("");
      refresh();
      if (ticket.status === "resolved") {
        onTicketUpdate({ ...ticket, status: "open" });
      }
      toast.success("Reply sent");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to send reply");
    } finally {
      setSending(false);
    }
  }, [replyBody, sending, ticket, sendReply, refresh, onTicketUpdate]);

  return (
    <aside className="flex min-h-0 w-96 shrink-0 flex-col overflow-hidden rounded-lg border bg-card text-card-foreground shadow-sm animate-in slide-in-from-right-3 fade-in duration-200"
    >
      {/* Header */}
      <div className="border-b p-4">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <h3 className="truncate font-medium text-base text-foreground">
              {ticket.subject}
            </h3>
            <div className="flex items-center gap-2 mt-1">
              <span className={`inline-flex items-center rounded-md border px-1.5 py-0.5 text-[10px] font-medium ${STATUS_STYLES[ticket.status]}`}>
                {STATUS_LABELS[ticket.status]}
              </span>
              <span className="text-[10px] text-muted-foreground">
                {format(new Date(ticket.createdAt), "MMM d, yyyy 'at' HH:mm")}
              </span>
            </div>
          </div>
          <button
            type="button"
            aria-label="Close details"
            className="rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
            onClick={onClose}
          >
            <X className="size-4" />
          </button>
        </div>
      </div>

      {/* Metadata */}
      <div className="border-b p-4">
        <dl className="grid grid-cols-2 gap-x-4 gap-y-3 text-sm">
          <div>
            <dt className="text-muted-foreground">Category</dt>
            <dd className="mt-1 font-medium">{ticket.category}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Priority</dt>
            <dd className="mt-1 font-medium">{ticket.priority}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Scope</dt>
            <dd className="mt-1 font-medium">{ticket.projectId ? "Project" : "Personal"}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">Updated</dt>
            <dd className="mt-1 font-medium">{format(new Date(ticket.updatedAt), "MMM d, HH:mm")}</dd>
          </div>
        </dl>
      </div>

      {/* Conversation */}
      <div ref={scrollRef} className="flex-1 min-h-0 overflow-y-auto p-4 space-y-3">
        {/* First message = the ticket description */}
        <div className="flex gap-2 justify-end">
          <div className="max-w-[85%] rounded-lg px-3 py-2 text-xs bg-primary text-primary-foreground">
            <p className="whitespace-pre-wrap">{ticket.description}</p>
            <p className="text-[10px] mt-1 text-primary-foreground/70">
              {format(new Date(ticket.createdAt), "MMM d, HH:mm")}
            </p>
          </div>
          <div className="size-6 rounded-full bg-muted flex items-center justify-center shrink-0 mt-0.5">
            <User className="size-3" />
          </div>
        </div>

        {/* Subsequent messages */}
        {loading && messages.length === 0 ? (
          <div className="flex items-center justify-center py-4 text-muted-foreground text-xs">
            <Loader2 className="size-4 animate-spin mr-2" /> Loading replies…
          </div>
        ) : error ? (
          <div className="text-center text-xs text-muted-foreground py-4">{error}</div>
        ) : (
          messages.map((msg) => (
            <div
              key={msg.id}
              className={`flex gap-2 ${msg.author === "customer" ? "justify-end" : "justify-start"}`}
            >
              {msg.author === "support" && (
                <div className="size-6 rounded-full bg-primary/10 flex items-center justify-center shrink-0 mt-0.5">
                  <Headphones className="size-3 text-primary" />
                </div>
              )}
              <div
                className={`max-w-[85%] rounded-lg px-3 py-2 text-xs ${
                  msg.author === "customer"
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted"
                }`}
              >
                <p className="whitespace-pre-wrap">{msg.body}</p>
                <p className={`text-[10px] mt-1 ${msg.author === "customer" ? "text-primary-foreground/70" : "text-muted-foreground"}`}>
                  {format(new Date(msg.createdAt), "MMM d, HH:mm")}
                </p>
              </div>
              {msg.author === "customer" && (
                <div className="size-6 rounded-full bg-muted flex items-center justify-center shrink-0 mt-0.5">
                  <User className="size-3" />
                </div>
              )}
            </div>
          ))
        )}
      </div>

      {/* Reply input */}
      <div className="border-t p-4">
        {isClosed ? (
          <div className="text-xs text-muted-foreground text-center py-2">
            This ticket is closed.{" "}
            <button onClick={onClose} className="text-primary hover:underline">
              Open a new ticket
            </button>{" "}
            for further assistance.
          </div>
        ) : (
          <div className="flex items-end gap-2">
            <Input
              placeholder="Type your reply…"
              value={replyBody}
              onChange={(e) => setReplyBody(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleSendReply();
                }
              }}
              disabled={sending}
              className="flex-1"
            />
            <Button
              variant="outline"
              size="icon"
              className="shrink-0"
              disabled={sending}
            >
              <Paperclip className="size-4" />
            </Button>
            <Button
              size="icon"
              className="shrink-0"
              onClick={handleSendReply}
              disabled={sending || !replyBody.trim()}
            >
              {sending ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Send className="size-4" />
              )}
            </Button>
          </div>
        )}
      </div>
    </aside>
  );
}
