"use client";

import { useState } from "react";
import { PageHeader } from "@/components/custom/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { env } from "@/lib/env";
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
} from "lucide-react";

type FaqItem = {
  question: string;
  answer: string;
  category: string;
};

const FAQS: FaqItem[] = [
  { question: "How do I get my API key?", answer: "Navigate to API Keys in your dashboard and click 'Generate key'. Choose the appropriate environment (Production, Staging, or Development).", category: "Getting Started" },
  { question: "How do I send my first notification?", answer: "Install the SDK for your language, initialize the client with your API key, and call sendNotification() with a recipient ID and message body.", category: "Getting Started" },
  { question: "What are the rate limits?", answer: "Rate limits depend on your plan. Free: 100 req/min, Starter: 500 req/min, Pro: 1000 req/min, Enterprise: Custom.", category: "Limits" },
  { question: "How do I add a new notification channel?", answer: "Go to Channels page, select the channel type you want to configure, and follow the setup wizard with your provider credentials.", category: "Channels" },
  { question: "How do webhooks work?", answer: "Webhooks deliver real-time event notifications to your endpoint. Configure the URL, select events, and we'll POST signed payloads to your server.", category: "Webhooks" },
  { question: "Can I test notifications before going live?", answer: "Yes, use the Staging or Development environment keys to send test notifications without affecting production metrics.", category: "Getting Started" },
];

export default function SupportPage() {
  const [search, setSearch] = useState("");
  const [ticketDialog, setTicketDialog] = useState(false);

  const filteredFaqs = search
    ? FAQS.filter((f) => f.question.toLowerCase().includes(search.toLowerCase()) || f.answer.toLowerCase().includes(search.toLowerCase()))
    : FAQS;

  return (
    <div className="space-y-6">
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

      <div className="grid gap-4 md:grid-cols-4">
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Documentation</span>
              <BookOpen className="size-4 text-muted-foreground" />
            </div>
            <a href={env.docs()} target="_blank" rel="noopener noreferrer" className="text-sm text-primary hover:underline mt-1 block">
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
            <a href={env.docs("/api")} target="_blank" rel="noopener noreferrer" className="text-sm text-primary hover:underline mt-1 block">
              View API docs →
            </a>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">Status</span>
              <CheckCircle2 className="size-4 text-success" />
            </div>
            <div className="text-sm mt-1">
              <span className="text-success font-medium">All systems operational</span>
            </div>
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
              value={search}
              onChange={(e) => setSearch(e.target.value)}
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
                { label: "Getting Started Guide", href: env.docs("/getting-started"), icon: BookOpen },
                { label: "API Reference", href: env.docs("/api"), icon: Zap },
                { label: "SDK Documentation", href: env.docs("/sdks"), icon: FileText },
                { label: "Changelog", href: env.docs("/changelog"), icon: FileText },
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

          <Card>
            <CardHeader>
              <CardTitle className="text-sm">System Status</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {[
                { name: "API Gateway", status: "healthy" },
                { name: "Email Delivery", status: "healthy" },
                { name: "SMS Delivery", status: "healthy" },
                { name: "Push Delivery", status: "healthy" },
                { name: "Webhooks", status: "healthy" },
                { name: "Dashboard", status: "healthy" },
              ].map((service) => (
                <div key={service.name} className="flex items-center justify-between text-sm">
                  <span>{service.name}</span>
                  <div className="flex items-center gap-1.5">
                    <span className="size-1.5 rounded-full bg-success" />
                    <span className="text-xs text-success">Operational</span>
                  </div>
                </div>
              ))}
            </CardContent>
          </Card>
        </div>
      </div>

      <Dialog open={ticketDialog} onOpenChange={setTicketDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Submit support ticket</DialogTitle>
            <DialogDescription>Describe your issue and our team will get back to you.</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Subject</label>
              <Input placeholder="Brief description of the issue" />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Category</label>
              <select className="w-full rounded-md border bg-transparent px-3 py-2 text-sm">
                <option>Technical Issue</option>
                <option>Billing</option>
                <option>Account Access</option>
                <option>Feature Request</option>
                <option>Integration Help</option>
                <option>Other</option>
              </select>
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Priority</label>
              <select className="w-full rounded-md border bg-transparent px-3 py-2 text-sm">
                <option>Low - General question</option>
                <option>Medium - Issue affecting workflow</option>
                <option>High - Service disruption</option>
                <option>Urgent - Production down</option>
              </select>
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">Description</label>
              <textarea
                placeholder="Provide details about your issue, including steps to reproduce, expected vs actual behavior, and any error messages..."
                className="w-full min-h-[150px] rounded-md border bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring resize-y"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setTicketDialog(false)}>Cancel</Button>
            <Button onClick={() => setTicketDialog(false)}>Submit ticket</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
