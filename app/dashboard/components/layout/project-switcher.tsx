"use client";

import { useEffect } from "react";
import { cn } from "@/lib/utils";
import { useProjectStore } from "@/store/project-store";
import { useEnvironmentStore } from "@/store/environment-store";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { ChevronsUpDown } from "lucide-react";

export function ProjectSwitcher({ collapsed = false }: { collapsed?: boolean }) {
  const { projects, currentProject, setCurrentProject, loadProjects } =
    useProjectStore();
  const syncEnv = useEnvironmentStore((s) => s.syncFromProject);

  useEffect(() => {
    void loadProjects();
  }, [loadProjects]);

  useEffect(() => {
    if (currentProject?.environment) syncEnv(currentProject.environment);
  }, [currentProject?.environment, syncEnv]);

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
    <DropdownMenuContent align="start" side={collapsed ? "right" : "bottom"} className="w-64">
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
    </DropdownMenuContent>
  );

  if (collapsed) {
    return (
      <DropdownMenu>
        <Tooltip>
          <TooltipTrigger render={trigger} />
          <TooltipContent side="right">
            {currentProject?.name ?? "Select project"}
          </TooltipContent>
        </Tooltip>
        {dropdown}
      </DropdownMenu>
    );
  }

  return (
    <DropdownMenu>
      {trigger}
      {dropdown}
    </DropdownMenu>
  );
}
