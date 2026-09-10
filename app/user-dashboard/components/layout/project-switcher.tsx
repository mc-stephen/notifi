"use client";

import { useEffect, useState, useCallback, useSyncExternalStore } from "react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { useProjectStore } from "@/store/project-store";
import { useEnvironmentStore } from "@/store/environment-store";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { ChevronsUpDown, Plus, Loader2 } from "lucide-react";

export function ProjectSwitcher({ collapsed = false }: { collapsed?: boolean }) {
  const { projects, currentProject, setCurrentProject, loadProjects, createProject } =
    useProjectStore();
  const syncEnv = useEnvironmentStore((s) => s.syncFromProject);

  const [createOpen, setCreateOpen] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const mounted = useSyncExternalStore(
    () => () => {},
    () => true,
    () => false,
  );

  useEffect(() => {
    useProjectStore.persist.rehydrate();
  }, []);

  useEffect(() => {
    void loadProjects();
  }, [loadProjects]);

  useEffect(() => {
    if (currentProject?.environment) syncEnv(currentProject.environment);
  }, [currentProject?.environment, syncEnv]);

  const handleCreate = useCallback(async () => {
    if (!newName.trim()) return;
    setSubmitting(true);
    try {
      await createProject(newName.trim(), newDescription.trim() || undefined);
      toast.success("Project created");
      setCreateOpen(false);
      setNewName("");
      setNewDescription("");
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed to create project");
    } finally {
      setSubmitting(false);
    }
  }, [newName, newDescription, createProject]);

  if (!mounted) return null;

  const trigger = (
    <DropdownMenuTrigger
      render={
        collapsed ? (
          <Button
            variant="ghost"
            className="h-8 w-full rounded-lg text-sidebar-foreground/70 hover:bg-sidebar-accent/60 hover:text-sidebar-foreground"
          />
        ) : (
          <Button
            variant="ghost"
            className="h-auto w-full gap-2 rounded-lg px-2.5 py-1.5 hover:bg-sidebar-accent/60 text-sidebar-foreground/80"
          />
        )
      }
    >
      <div className="flex size-6 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary text-xs font-bold">
        {currentProject?.name?.charAt(0) ?? "P"}
      </div>
      {!collapsed && (
        <span className="text-sm font-medium max-w-[160px] truncate">
          {currentProject?.name ?? "Select project"}
        </span>
      )}
      {!collapsed && (
        <ChevronsUpDown className="ml-auto size-3.5 shrink-0 text-muted-foreground" />
      )}
    </DropdownMenuTrigger>
  );

  const dropdown = (
    <DropdownMenuContent align="start" side="right" className="w-64">
      <DropdownMenuLabel>Project</DropdownMenuLabel>
      <DropdownMenuSeparator />
      {projects.map((project) => (
        <DropdownMenuItem
          key={project.id}
          onClick={() => setCurrentProject(project)}
          className={cn(project.id === currentProject?.id && "bg-accent")}
        >
          <div className="flex size-6 items-center justify-center rounded-md bg-primary/10 text-primary text-xs font-bold">
            {project.name.charAt(0)}
          </div>
          <div className="flex flex-col">
            <span className="text-sm">{project.name}</span>
            {project.description && (
              <span className="text-xs text-muted-foreground truncate max-w-[200px]">
                {project.description}
              </span>
            )}
          </div>
        </DropdownMenuItem>
      ))}
      <DropdownMenuSeparator />
      <DropdownMenuItem onClick={() => setCreateOpen(true)}>
        <Plus className="size-4 mr-2" />
        Create new project
      </DropdownMenuItem>
    </DropdownMenuContent>
  );

  if (collapsed) {
    return (
      <>
        <DropdownMenu>
          <Tooltip>
            <TooltipTrigger render={trigger} />
            <TooltipContent side="right">
              {currentProject?.name ?? "Select project"}
            </TooltipContent>
          </Tooltip>
          {dropdown}
        </DropdownMenu>

        <Dialog open={createOpen} onOpenChange={setCreateOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Create new project</DialogTitle>
              <DialogDescription>
                Give your project a name to get started.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="project-name">Name</Label>
                <Input
                  id="project-name"
                  placeholder="My App"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleCreate();
                  }}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="project-desc">Description (optional)</Label>
                <Input
                  id="project-desc"
                  placeholder="What is this project for?"
                  value={newDescription}
                  onChange={(e) => setNewDescription(e.target.value)}
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setCreateOpen(false)}>
                Cancel
              </Button>
              <Button
                onClick={handleCreate}
                disabled={submitting || !newName.trim()}
              >
                {submitting ? (
                  <Loader2 className="size-3.5 animate-spin mr-1.5" />
                ) : null}
                {submitting ? "Creating…" : "Create project"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </>
    );
  }

  return (
    <>
      <DropdownMenu>
        {trigger}
        {dropdown}
      </DropdownMenu>

      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Create new project</DialogTitle>
              <DialogDescription>
                Give your project a name to get started.
              </DialogDescription>
            </DialogHeader>
            <div className="space-y-4 py-4">
              <div className="space-y-2">
                <Label htmlFor="project-name">Name</Label>
                <Input
                  id="project-name"
                  placeholder="My App"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleCreate();
                  }}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="project-desc">Description (optional)</Label>
                <Input
                  id="project-desc"
                  placeholder="What is this project for?"
                  value={newDescription}
                  onChange={(e) => setNewDescription(e.target.value)}
                />
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={() => setCreateOpen(false)}>
                Cancel
              </Button>
              <Button
                onClick={handleCreate}
                disabled={submitting || !newName.trim()}
              >
                {submitting ? (
                  <Loader2 className="size-3.5 animate-spin mr-1.5" />
                ) : null}
                {submitting ? "Creating…" : "Create project"}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
    </>
  );
}
