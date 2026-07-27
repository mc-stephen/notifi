"use client";

import { useState, Suspense } from "react";
import { useRouter } from "next/navigation";
import { cn } from "@/lib/utils";
import { useAuthStore } from "@/store/auth-store";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
} from "@/components/ui/dropdown-menu";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Sheet, SheetTrigger, SheetContent, SheetTitle } from "@/components/ui/sheet";
import { Sidebar } from "./sidebar";
import { NavigationProgress } from "@/components/custom/navigation-progress";
import { useOrgStore } from "@/store/org-store";
import { useProjectStore } from "@/store/project-store";
import { useEnvironmentStore } from "@/store/environment-store";
import { useUIStore } from "@/store/ui-store";
import { ENVIRONMENT_COLORS, ENVIRONMENT_LABELS } from "@/lib/constants";
import type { Environment } from "@/lib/types";
import {
  Menu,
  Search,
  ChevronsUpDown,
  Bell,
  Moon,
  Sun,
  LogOut,
  Settings,
  User,
} from "lucide-react";
import { useTheme } from "next-themes";

function OrgSwitcher() {
  const { organizations, currentOrg, setCurrentOrg } = useOrgStore();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button variant="ghost" className="gap-2 px-2 py-1.5 h-auto" />
        }
      >
        <div className="flex size-6 items-center justify-center rounded bg-primary/10 text-primary text-xs font-bold">
          {currentOrg?.name?.charAt(0) ?? "N"}
        </div>
        <span className="hidden sm:inline text-sm font-medium max-w-[120px] truncate">{currentOrg?.name ?? "Select org"}</span>
        <ChevronsUpDown className="size-3.5 text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-56">
        <DropdownMenuLabel>Organization</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {organizations.map((org) => (
          <DropdownMenuItem
            key={org.id}
            onClick={() => setCurrentOrg(org)}
            className={cn(org.id === currentOrg?.id && "bg-accent")}
          >
            <div className="flex size-6 items-center justify-center rounded bg-primary/10 text-primary text-xs font-bold">
              {org.name.charAt(0)}
            </div>
            <div className="flex flex-col">
              <span className="text-sm">{org.name}</span>
              <span className="text-xs text-muted-foreground capitalize">{org.plan}</span>
            </div>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function ProjectSwitcher() {
  const { projects, currentProject, setCurrentProject } = useProjectStore();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button variant="ghost" size="sm" className="gap-1.5 px-2" />
        }
      >
        <span className="text-sm font-medium max-w-[140px] truncate">{currentProject?.name ?? "Select project"}</span>
        <ChevronsUpDown className="size-3 text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-56">
        <DropdownMenuLabel>Project</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {projects.map((project) => (
          <DropdownMenuItem
            key={project.id}
            onClick={() => setCurrentProject(project)}
            className={cn(project.id === currentProject?.id && "bg-accent")}
          >
            <div className="flex flex-col">
              <span className="text-sm">{project.name}</span>
              {project.description && (
                <span className="text-xs text-muted-foreground truncate max-w-[200px]">{project.description}</span>
              )}
            </div>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function EnvironmentSwitcher() {
  const { currentEnvironment, setEnvironment } = useEnvironmentStore();
  const envs: Environment[] = ["development", "staging", "production"];

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button variant="ghost" size="sm" className="gap-1.5 px-2" />
        }
      >
        <Badge variant="outline" className={cn("gap-1.5 font-mono text-[10px] font-semibold uppercase tracking-wider", ENVIRONMENT_COLORS[currentEnvironment])}>
          {ENVIRONMENT_LABELS[currentEnvironment]}
        </Badge>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="center">
        <DropdownMenuRadioGroup value={currentEnvironment} onValueChange={(v) => setEnvironment(v as Environment)}>
          {envs.map((env) => (
            <DropdownMenuRadioItem key={env} value={env}>
              <Badge variant="outline" className={cn("gap-1.5 font-mono text-[10px] font-semibold uppercase tracking-wider", ENVIRONMENT_COLORS[env])}>
                {ENVIRONMENT_LABELS[env]}
              </Badge>
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function SearchButton() {
  const { setCommandOpen } = useUIStore();

  return (
    <Button
      variant="outline"
      size="sm"
      className="gap-2 text-muted-foreground h-8 w-48 justify-start"
      onClick={() => setCommandOpen(true)}
    >
      <Search className="size-3.5" />
      <span className="text-sm">Search...</span>
      <kbd className="ml-auto pointer-events-none inline-flex h-5 select-none items-center gap-0.5 rounded border bg-muted px-1.5 text-[10px] font-medium text-muted-foreground">
        <span className="text-xs">⌘</span>K
      </kbd>
    </Button>
  );
}

function UserMenu() {
  const router = useRouter();
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);

  const initials = user?.name
    ? user.name
        .split(" ")
        .map((n) => n[0])
        .join("")
        .toUpperCase()
        .slice(0, 2)
    : "?";

  const handleLogout = () => {
    logout();
    document.cookie = "session_token=; path=/; max-age=0";
    router.push("/auth/login");
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button variant="ghost" size="icon-sm" className="rounded-full" />
        }
      >
        <Avatar className="size-7">
          <AvatarFallback className="bg-primary/10 text-primary text-xs font-medium">{initials}</AvatarFallback>
        </Avatar>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48">
        <DropdownMenuLabel>
          <div className="flex flex-col">
            <span className="text-sm">{user?.name ?? "Unknown"}</span>
            <span className="text-xs text-muted-foreground font-normal">{user?.email ?? ""}</span>
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={() => router.push("/profile")}>
          <User className="size-4" />
          Profile
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => router.push("/settings")}>
          <Settings className="size-4" />
          Settings
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem className="text-destructive" onClick={handleLogout}>
          <LogOut className="size-4" />
          Sign out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  // Avoid hydration mismatch
  if (typeof window !== "undefined" && !mounted) {
    setMounted(true);
  }

  return (
    <Button
      variant="ghost"
      size="icon-sm"
      onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
      aria-label="Toggle theme"
    >
      {theme === "dark" ? <Sun className="size-4" /> : <Moon className="size-4" />}
    </Button>
  );
}

export function Topbar() {
  const [mobileOpen, setMobileOpen] = useState(false);

  return (
    <>
      <header className="sticky top-0 z-40 flex h-14 items-center gap-4 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 px-4 lg:px-6 relative">
        {/* Mobile menu */}
        <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
          <SheetTrigger
            render={
              <Button variant="ghost" size="icon-sm" className="lg:hidden" />
            }
          >
            <Menu className="size-5" />
            <span className="sr-only">Toggle menu</span>
          </SheetTrigger>
          <SheetContent side="left" className="w-60 p-0" showCloseButton={false}>
            <SheetTitle className="sr-only">Navigation</SheetTitle>
            <Sidebar mobile onNavigate={() => setMobileOpen(false)} />
          </SheetContent>
        </Sheet>

        {/* Org + Project switchers */}
        <div className="flex items-center gap-1">
          <OrgSwitcher />
          <span className="text-muted-foreground">/</span>
          <ProjectSwitcher />
          <EnvironmentSwitcher />
        </div>

        {/* Right side */}
        <div className="ml-auto flex items-center gap-2">
          <div className="hidden md:block">
            <SearchButton />
          </div>
          <Button variant="ghost" size="icon-sm" className="relative">
            <Bell className="size-4" />
            <span className="absolute -top-0.5 -right-0.5 size-2 rounded-full bg-primary" />
          </Button>
          <ThemeToggle />
          <UserMenu />
        </div>

        {/* Navigation progress indicator */}
        <Suspense fallback={null}>
          <NavigationProgress className="absolute bottom-0 left-0 right-0 z-50" />
        </Suspense>
      </header>
    </>
  );
}
