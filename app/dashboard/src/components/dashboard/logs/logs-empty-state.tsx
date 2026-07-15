"use client";

import { EmptyState } from "@/components/dashboard/shared/empty-state";
import { ScrollText, Send } from "lucide-react";
import { Button } from "@/components/ui/button";

export function LogsEmptyState() {
  return (
    <EmptyState
      icon={<ScrollText className="h-6 w-6" />}
      title="No activity yet"
      description="Your notification logs will appear here once you send your first request."
      action={
        <Button size="sm">
          <Send className="h-3.5 w-3.5" />
          Send your first test notification
        </Button>
      }
    />
  );
}
