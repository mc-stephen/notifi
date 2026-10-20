"use client";

import { useState, useSyncExternalStore } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { useAuthStore } from "@/store/auth-store";
import { useProjectStore } from "@/store/project-store";
import { useUIStore } from "@/store/ui-store";
import { api, ApiError } from "@/lib/api";
import type { Project } from "@/lib/types";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import {
  Sheet,
  SheetTrigger,
  SheetContent,
  SheetTitle,
} from "@/components/ui/sheet";
import { NotificationMenu } from "@/components/custom/notification-menu";
import { Sidebar } from "./sidebar";
import {
  Menu,
  Search,
  Moon,
  Sun,
  LogOut,
  ShieldCheck,
  User,
  Loader2,
} from "lucide-react";
import { useTheme } from "next-themes";

export function SidebarSearch({ collapsed }: { collapsed: boolean }) {
  const { setCommandOpen } = useUIStore();

  if (collapsed) {
    return (
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={() => setCommandOpen(true)}
        aria-label="Search"
        className="w-full text-white/80 hover:bg-white/10 hover:text-white"
      >
        <Search className="size-4" />
      </Button>
    );
  }

  return (
    <Button
      variant="outline"
      size="sm"
      className="h-8 w-full justify-start gap-2 border-white/25 bg-white/10 text-white/80 hover:bg-white/15 hover:text-white"
      onClick={() => setCommandOpen(true)}
    >
      <Search className="size-3.5" />
      <span className="text-sm">Search...</span>
      <kbd className="ml-auto pointer-events-none inline-flex h-5 select-none items-center gap-0.5 rounded border border-white/20 bg-white/10 px-1.5 text-[10px] font-medium text-white/70">
        <span className="text-xs">⌘</span>K
      </kbd>
    </Button>
  );
}

/** Quick toggle for the current project's active switch. */
export function CompactActiveSwitch() {
  const currentProject = useProjectStore((s) => s.currentProject);
  const updateProject = useProjectStore((s) => s.updateProject);
  const [pending, setPending] = useState(false);
  // Mount guard for SSR: the project store fills in client-side only.
  // Pre-mount we render a store-free mirror of the pills so the server
  // HTML and the hydration render match by construction — no
  // store-driven attributes race the hydration window.
  const mounted = useSyncExternalStore(
    () => () => {},
    () => true,
    () => false,
  );

  const active = currentProject?.active ?? false;

  const setActive = async (next: boolean) => {
    if (!currentProject || pending || next === currentProject.active) return;
    setPending(true);
    try {
      const { project } = await api<{ project: Project }>(
        `/app/projects/${currentProject.id}/active`,
        { method: "PATCH", body: JSON.stringify({ active: next }) },
      );
      updateProject({ ...currentProject, active: project.active });
    } catch (e) {
      const detail = e instanceof ApiError ? e.message : "Check your connection and try again.";
      toast.error("Couldn't change project state", { description: detail });
    } finally {
      setPending(false);
    }
  };

  if (!mounted) {
    return (
      <div
        className="flex min-w-0 flex-1 items-center gap-1 rounded-full bg-black/15 p-1"
        role="radiogroup"
      >
        <button
          type="button"
          role="radio"
          aria-checked={true}
          title="Active — notifications deliver normally"
          aria-label="Active"
          className={cn(
            "flex min-w-0 flex-1 items-center justify-center gap-1 rounded-full px-2 py-1 text-xs font-medium transition-colors disabled:opacity-60",
            "bg-white text-sidebar-primary-foreground shadow-sm",
          )}
        >
          <span className="size-1.5 shrink-0 rounded-full bg-success" />
          <span className="truncate">Active</span>
        </button>
        <button
          type="button"
          role="radio"
          aria-checked={false}
          title="Inactive — all delivery stopped"
          aria-label="Inactive"
          className={cn(
            "flex min-w-0 flex-1 items-center justify-center gap-1 rounded-full px-2 py-1 text-xs font-medium transition-colors disabled:opacity-60",
            "text-white/75 hover:text-white",
          )}
        >
          <span className="size-1.5 shrink-0 rounded-full bg-destructive" />
          <span className="truncate">Off</span>
        </button>
      </div>
    );
  }

  return (
    <div
      className="flex min-w-0 flex-1 items-center gap-1 rounded-full bg-black/15 p-1"
      role="radiogroup"
      aria-busy={pending || undefined}
    >
      {([true, false] as const).map((value) => {
        const selected = value === active;
        return (
          <button
            key={value ? "active" : "inactive"}
            type="button"
            role="radio"
            aria-checked={selected}
            title={value ? "Active — notifications deliver normally" : "Inactive — all delivery stopped"}
            aria-label={value ? "Active" : "Inactive"}
            disabled={pending || !currentProject}
            onClick={() => {
              void setActive(value);
            }}
            className={cn(
              "flex min-w-0 flex-1 items-center justify-center gap-1 rounded-full px-2 py-1 text-xs font-medium transition-colors disabled:opacity-60",
              selected
                ? "bg-white text-sidebar-primary-foreground shadow-sm"
                : "text-white/75 hover:text-white",
            )}
          >
            {pending && selected ? (
              <Loader2 className="size-1.5 shrink-0 animate-spin" />
            ) : (
              <span
                className={cn(
                  "size-1.5 shrink-0 rounded-full",
                  value ? "bg-success" : "bg-destructive",
                )}
              />
            )}
            <span className="truncate">{value ? "Active" : "Off"}</span>
          </button>
        );
      })}
    </div>
  );
}

export function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const mounted = useSyncExternalStore(
    () => () => {},
    () => true,
    () => false,
  );

  return (
    <Button
      variant="ghost"
      size="icon-sm"
      onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
      aria-label="Toggle theme"
      className="shrink-0 text-white/80 hover:bg-white/10 hover:text-white"
    >
      {mounted ? (
        theme === "dark" ? (
          <Sun className="size-4" />
        ) : (
          <Moon className="size-4" />
        )
      ) : (
        <Sun className="size-4" />
      )}
    </Button>
  );
}

export function UserMenu() {
  const router = useRouter();
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);
  const [loggingOut, setLoggingOut] = useState(false);

  const initials = user?.name
    ? user.name
        .split(" ")
        .map((n) => n[0])
        .join("")
        .toUpperCase()
        .slice(0, 2)
    : "?";

  // Await the API call: the httpOnly session cookie is cleared by the
  // server's response — navigating before that lands gets bounced back to
  // `/` by the auth proxy.
  const handleLogout = async () => {
    if (loggingOut) return;
    setLoggingOut(true);
    await logout();
    router.push("/auth/login");
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            size="icon-sm"
            className="shrink-0 rounded-full text-white/80 hover:bg-white/10 hover:text-white aria-expanded:bg-white/10 aria-expanded:text-white"
          />
        }
      >
        <Avatar className="size-7">
          <AvatarFallback className="bg-white/25 text-xs font-semibold text-white ring-1 ring-white/30">
            {initials}
          </AvatarFallback>
        </Avatar>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>
          <div className="flex min-w-0 flex-col">
            <span className="truncate text-sm">{user?.name ?? "Unknown"}</span>
            <span className="truncate text-xs font-normal text-muted-foreground">
              {user?.email ?? ""}
            </span>
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={() => router.push("/profile")}>
          <User className="size-4" />
          Profile
        </DropdownMenuItem>
        <DropdownMenuItem onClick={() => router.push("/security")}>
          <ShieldCheck className="size-4" />
          Security
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          variant="destructive"
          onClick={handleLogout}
          disabled={loggingOut}
        >
          <LogOut className="size-4" />
          {loggingOut ? "Signing out..." : "Sign out"}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** Horizontal utility row pinned above the project switcher. */
export function SidebarUtilityRow({ collapsed }: { collapsed: boolean }) {
  if (collapsed) {
    return (
      <div className="flex flex-col items-center gap-1">
        <NotificationMenu />
        <UserMenu />
        <ThemeToggle />
      </div>
    );
  }

  return (
    <div className="flex items-center gap-1.5">
      <CompactActiveSwitch />
      <NotificationMenu />
      <UserMenu />
      <ThemeToggle />
    </div>
  );
}

/** Minimal mobile-only strip: drawer menu + search (header replacement). */
export function MobileStrip() {
  const [mobileOpen, setMobileOpen] = useState(false);
  const { setCommandOpen } = useUIStore();

  return (
    <div className="flex h-12 shrink-0 items-center gap-2 border-b border-sidebar-border bg-sidebar px-3 lg:hidden">
      <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
        <SheetTrigger
          render={
            <Button
              variant="ghost"
              size="icon-sm"
              className="text-white/80 hover:bg-white/10 hover:text-white"
            />
          }
        >
          <Menu className="size-5" />
          <span className="sr-only">Toggle menu</span>
        </SheetTrigger>
        <SheetContent
          side="left"
          className="w-60 gap-0 p-0"
          showCloseButton={false}
        >
          <SheetTitle className="sr-only">Navigation</SheetTitle>
          <Sidebar mobile onNavigate={() => setMobileOpen(false)} />
        </SheetContent>
      </Sheet>
      <Button
        variant="ghost"
        size="icon-sm"
        onClick={() => setCommandOpen(true)}
        aria-label="Search"
        className="text-white/80 hover:bg-white/10 hover:text-white"
      >
        <Search className="size-4" />
      </Button>
    </div>
  );
}
