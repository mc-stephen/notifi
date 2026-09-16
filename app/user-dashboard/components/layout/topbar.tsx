"use client";

import { useState, useSyncExternalStore, Suspense } from "react";
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
} from "@/components/ui/dropdown-menu";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Sheet, SheetTrigger, SheetContent, SheetTitle } from "@/components/ui/sheet";
import { Sidebar } from "./sidebar";
import { NavigationProgress } from "@/components/custom/navigation-progress";
import { NotificationMenu } from "@/components/custom/notification-menu";
import { useProjectStore } from "@/store/project-store";
import { useEnvironmentStore } from "@/store/environment-store";
import { useUIStore } from "@/store/ui-store";
import { ENVIRONMENT_DOTS, ENVIRONMENT_LABELS } from "@/lib/constants";
import type { Environment } from "@/lib/types";
import {
  Menu,
  Search,
  Moon,
  Sun,
  LogOut,
  Settings,
  User,
  Loader2,
} from "lucide-react";
import { useTheme } from "next-themes";

function EnvironmentSwitcher() {
  const { currentEnvironment, pending, switchEnvironment } = useEnvironmentStore();
  const currentProject = useProjectStore((s) => s.currentProject);
  const envs: Environment[] = ["development", "production"];

  return (
    <div className="flex items-center gap-1 rounded-full bg-black/15 p-1" role="radiogroup" aria-busy={pending || undefined}>
      {envs.map((env) => {
        const active = env === currentEnvironment;
        return (
          <button
            key={env}
            type="button"
            role="radio"
            aria-checked={active}
            disabled={pending}
            onClick={() => {
              if (currentProject) void switchEnvironment(currentProject.id, env);
            }}
            className={cn(
              "flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium transition-colors disabled:opacity-60",
              active
                ? "bg-white text-sidebar-primary-foreground shadow-sm"
                : "text-white/65 hover:text-white",
            )}
          >
            {pending && active ? (
              <Loader2 className="size-1.5 animate-spin" />
            ) : (
              <span className={cn("size-1.5 rounded-full", ENVIRONMENT_DOTS[env])} />
            )}
            {ENVIRONMENT_LABELS[env]}
          </button>
        );
      })}
    </div>
  );
}

function SearchButton() {
  const { setCommandOpen } = useUIStore();

  return (
    <Button
      variant="outline"
      size="sm"
      className="h-8 w-48 justify-start gap-2 border-white/25 bg-white/10 text-white/80 hover:bg-white/15 hover:text-white"
      onClick={() => setCommandOpen(true)}
    >
      <Search className="size-3.5" />
      <span className="text-sm">Search...</span>
      <kbd className="ml-auto pointer-events-none inline-flex h-5 select-none items-center gap-0.5 rounded border border-white/20 bg-white/10 px-1.5 text-[10px] font-medium text-white/60">
        <span className="text-xs">⌘</span>K
      </kbd>
    </Button>
  );
}

function UserMenu() {
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
          <Button variant="ghost" size="icon-sm" className="rounded-full text-white/80 hover:bg-white/10 hover:text-white aria-expanded:bg-white/10 aria-expanded:text-white" />
        }
      >
        <Avatar className="size-7">
          <AvatarFallback className="bg-white/20 text-white text-xs font-medium">{initials}</AvatarFallback>
        </Avatar>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>
          <div className="flex min-w-0 flex-col">
            <span className="truncate text-sm">{user?.name ?? "Unknown"}</span>
            <span className="truncate text-xs text-muted-foreground font-normal">{user?.email ?? ""}</span>
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
        <DropdownMenuItem
          className="text-destructive"
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

function ThemeToggle() {
  const { theme, setTheme } = useTheme();
  const mounted = useSyncExternalStore(
    () => () => {},
    () => true,
    () => false
  );

  return (
    <Button
      variant="ghost"
      size="icon-sm"
      onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
      aria-label="Toggle theme"
      className="text-white/70 hover:bg-white/10 hover:text-white"
    >
      {mounted
        ? (theme === "dark" ? <Sun className="size-4" /> : <Moon className="size-4" />)
        : <Sun className="size-4" />}
    </Button>
  );
}

export function Topbar() {
  const [mobileOpen, setMobileOpen] = useState(false);

  return (
    <>
      <header className="sticky top-0 z-40 flex h-14 items-center gap-4 border-b border-sidebar-border bg-sidebar px-4 lg:px-6 relative">
        {/* Mobile menu */}
        <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
          <SheetTrigger
            render={
              <Button variant="ghost" size="icon-sm" className="text-white/80 hover:bg-white/10 hover:text-white lg:hidden" />
            }
          >
            <Menu className="size-5" />
            <span className="sr-only">Toggle menu</span>
          </SheetTrigger>
          <SheetContent side="left" className="w-60 p-0 gap-0" showCloseButton={false}>
            <SheetTitle className="sr-only">Navigation</SheetTitle>
            <Sidebar mobile onNavigate={() => setMobileOpen(false)} />
          </SheetContent>
        </Sheet>

        {/* Search (left) */}
        <div className="hidden md:block">
          <SearchButton />
        </div>

        {/* Environment switch (centered) */}
        <div className="absolute left-1/2 hidden -translate-x-1/2 md:block">
          <EnvironmentSwitcher />
        </div>

        {/* Right side */}
        <div className="ml-auto flex items-center gap-2">
          <NotificationMenu />
          <ThemeToggle />
          <UserMenu />
        </div>

        {/* Navigation progress indicator */}
        <Suspense fallback={null}>
          <NavigationProgress className="absolute bottom-0 left-0 right-0 z-50" />
        </Suspense>

        {/* Film grain over the brand chrome */}
        <div
          aria-hidden="true"
          className="bg-noise pointer-events-none absolute inset-0 opacity-[0.08]"
        />
      </header>
    </>
  );
}
