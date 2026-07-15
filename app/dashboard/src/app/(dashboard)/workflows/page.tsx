"use client";

import { PageHeader } from "@/components/dashboard/shared/page-header";
import { EmptyState } from "@/components/dashboard/shared/empty-state";
import { Workflow, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";

export default function WorkflowsPage() {
  return (
    <div className="mx-auto w-full max-w-7xl space-y-6 p-5">
      <PageHeader
        title="Workflows & Templates"
        description="Map and edit notification routing workflows."
        action={
          <Button size="sm">
            <Plus className="h-3.5 w-3.5" />
            Create Workflow
          </Button>
        }
      />

      <EmptyState
        icon={<Workflow className="h-6 w-6" />}
        title="No workflows yet"
        description="Create your first workflow to define how notifications are routed, filtered, and delivered."
        action={
          <Button size="sm">
            <Plus className="h-3.5 w-3.5" />
            Create Workflow
          </Button>
        }
      />
    </div>
  );
}
