"use client";

import { useState, useSyncExternalStore } from "react";
import { Loader2, Pause, Play } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { api, ApiError } from "@/lib/api";
import type { Project } from "@/lib/types";
import { useProjectStore } from "@/store/project-store";

function subscribeNoop() {
  return () => {};
}

/** One-line strip shown while the current project's active switch is off.
 * Not dismissible — "Resume delivery" posts the same PATCH as the topbar
 * switch, and the banner disappears on success. */
export function ActiveBanner() {
  const currentProject = useProjectStore((s) => s.currentProject);
  const updateProject = useProjectStore((s) => s.updateProject);
  const [resuming, setResuming] = useState(false);
  // Mount guard for SSR: project state only hydrates on the client.
  const mounted = useSyncExternalStore(subscribeNoop, () => true, () => false);

  if (!mounted || !currentProject || currentProject.active) return null;

  const handleResume = async () => {
    if (resuming) return;
    setResuming(true);
    try {
      const { project } = await api<{ project: Project }>(
        `/app/projects/${currentProject.id}/active`,
        { method: "PATCH", body: JSON.stringify({ active: true }) },
      );
      updateProject({ ...currentProject, active: project.active });
    } catch (e) {
      toast.error("Couldn't resume delivery", {
        description:
          e instanceof ApiError
            ? e.message
            : "Check your connection and try again.",
      });
    } finally {
      setResuming(false);
    }
  };

  return (
    <div className="flex items-center justify-between gap-3 border-b border-warning/30 bg-warning/10 px-4 py-2.5 lg:px-6">
      <p className="flex min-w-0 items-center gap-2 text-sm text-amber-700 dark:text-amber-400">
        <Pause className="size-3.5 shrink-0" />
        <span className="truncate">
          This project is inactive — delivery is paused and API calls are rejected.
        </span>
      </p>

      <Button
        size="sm"
        variant="outline"
        onClick={handleResume}
        disabled={resuming}
        className="shrink-0 bg-popover shadow-sm"
      >
        {resuming ? (
          <>
            <Loader2 className="mr-2 size-3 animate-spin" />
            Resuming…
          </>
        ) : (
          <>
            <Play className="mr-2 size-3" />
            Resume delivery
          </>
        )}
      </Button>
    </div>
  );
}
