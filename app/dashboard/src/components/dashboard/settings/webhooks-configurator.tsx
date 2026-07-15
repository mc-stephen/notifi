"use client";

import { useState } from "react";
import { Plus, Trash2, ExternalLink } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { WebhookForm } from "./webhook-form";
import type { Webhook } from "@/lib/types";

interface WebhooksConfiguratorProps {
  webhooks: Webhook[];
  availableEvents: readonly string[];
  onAdd: (url: string, events: string[]) => void;
  onRemove: (id: string) => void;
}

export function WebhooksConfigurator({
  webhooks,
  availableEvents,
  onAdd,
  onRemove,
}: WebhooksConfiguratorProps) {
  const [showForm, setShowForm] = useState(false);

  const handleAdd = (url: string, events: string[]) => {
    onAdd(url, events);
    setShowForm(false);
  };

  return (
    <div className="space-y-4">
      {webhooks.length > 0 && (
        <div className="space-y-3">
          {webhooks.map((wh) => (
            <div key={wh.id} className="card-lift rounded-xl border border-border/40 bg-card p-4">
              <div className="flex items-start justify-between">
                <div className="space-y-2 flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <a
                      href={wh.url}
                      target="_blank"
                      rel="noreferrer"
                      className="text-sm font-medium text-primary hover:underline flex items-center gap-1"
                    >
                      {wh.url}
                      <ExternalLink className="h-3 w-3" />
                    </a>
                    <Badge variant={wh.active ? "default" : "secondary"} className="text-[10px]">
                      {wh.active ? "Active" : "Inactive"}
                    </Badge>
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {wh.events.map((evt) => (
                      <Badge key={evt} variant="outline" className="text-[10px]">
                        {evt}
                      </Badge>
                    ))}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Created {new Date(wh.createdAt).toLocaleDateString()}
                  </p>
                </div>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => onRemove(wh.id)}
                  className="text-muted-foreground hover:text-red-500"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      {showForm ? (
        <div className="rounded-xl border border-border/40 bg-card p-4">
          <WebhookForm
            availableEvents={availableEvents}
            onSubmit={handleAdd}
            onCancel={() => setShowForm(false)}
          />
        </div>
      ) : (
        <Button variant="outline" onClick={() => setShowForm(true)}>
          <Plus className="h-3.5 w-3.5" />
          Add Webhook Endpoint
        </Button>
      )}

      {webhooks.length === 0 && !showForm && (
        <p className="text-sm text-muted-foreground text-center py-4">
          No webhooks configured. Add one to receive event notifications.
        </p>
      )}
    </div>
  );
}
