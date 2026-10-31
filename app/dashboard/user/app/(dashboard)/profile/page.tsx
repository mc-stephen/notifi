"use client";

import { useRef, useState, useSyncExternalStore } from "react";
import { useRouter } from "next/navigation";
import { useTheme } from "next-themes";
import { cn } from "@/lib/utils";
import {
  User,
  ShieldCheck,
  ShieldOff,
  BadgeCheck,
  FolderKanban,
  Palette,
  Sun,
  Moon,
  Laptop,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Avatar, AvatarImage, AvatarFallback } from "@/components/ui/avatar";
import { PageHeader } from "@/components/custom/page-header";
import { useAuthStore } from "@/store/auth-store";
import { useProjectStore } from "@/store/project-store";
import { initials } from "@/lib/utils";
import { ActiveBadge } from "@/components/custom/active-badge";

/** Rotating tile tones for project rows — teal, blue, green, amber. */
const PROJECT_TILE_TONES = [
  "bg-primary/10 text-primary",
  "bg-info/10 text-info",
  "bg-success/10 text-success",
  "bg-warning/10 text-warning",
] as const;

const THEMES = [
  { id: "light", label: "Light", icon: Sun },
  { id: "dark", label: "Dark", icon: Moon },
  { id: "system", label: "System", icon: Laptop },
] as const;

export default function ProfilePage() {
  const router = useRouter();
  const user = useAuthStore((s) => s.user);
  const projects = useProjectStore((s) => s.projects);
  const currentProject = useProjectStore((s) => s.currentProject);
  const setCurrentProject = useProjectStore((s) => s.setCurrentProject);

  const { theme, setTheme } = useTheme();
  const themeMounted = useSyncExternalStore(
    () => () => {},
    () => true,
    () => false,
  );

  // Personal info
  const [name, setName] = useState(user?.name ?? "");
  const [email, setEmail] = useState(user?.email ?? "");
  const [savingProfile, setSavingProfile] = useState(false);
  const [profileSuccess, setProfileSuccess] = useState(false);

  const userInitials = initials(user?.name);

  // Avatar — the profile card's round preview drives the whole trio:
  // upload raw bytes, remove; both re-sync from /me and bump the
  // cache-busting ?v so the image refetches.
  const [avatarVersion, setAvatarVersion] = useState(0);
  const [avatarBusy, setAvatarBusy] = useState(false);
  const [avatarError, setAvatarError] = useState<string | null>(null);
  const avatarInputRef = useRef<HTMLInputElement>(null);

  const avatarProblem = async (res: Response, fallback: string) => {
    const body = (await res.json().catch(() => null)) as { detail?: string } | null;
    return body?.detail ?? `${fallback} (${res.status}).`;
  };

  const handleAvatarUpload = async (file: File) => {
    if (!["image/jpeg", "image/png", "image/webp"].includes(file.type)) {
      setAvatarError("Use a JPEG, PNG or WebP image.");
      return;
    }
    if (file.size > 2 * 1024 * 1024) {
      setAvatarError("Image must be 2 MiB or smaller.");
      return;
    }
    setAvatarBusy(true);
    setAvatarError(null);
    try {
      const res = await fetch("/app/auth/profile/avatar", {
        method: "POST",
        headers: { "Content-Type": file.type },
        body: file,
      });
      if (!res.ok) throw new Error(await avatarProblem(res, "Upload failed"));
      setAvatarVersion((v) => v + 1);
      await useAuthStore.getState().fetchMe();
    } catch (err) {
      setAvatarError(err instanceof Error ? err.message : "Upload failed.");
    } finally {
      setAvatarBusy(false);
    }
  };

  const handleAvatarRemove = async () => {
    setAvatarBusy(true);
    setAvatarError(null);
    try {
      const res = await fetch("/app/auth/profile/avatar", { method: "DELETE" });
      if (!res.ok) throw new Error(await avatarProblem(res, "Remove failed"));
      setAvatarVersion((v) => v + 1);
      await useAuthStore.getState().fetchMe();
    } catch (err) {
      setAvatarError(err instanceof Error ? err.message : "Remove failed.");
    } finally {
      setAvatarBusy(false);
    }
  };

  const handleSaveProfile = async () => {
    setSavingProfile(true);
    setProfileSuccess(false);
    await new Promise((r) => setTimeout(r, 600));
    // Mock update — in production this would call an API
    useAuthStore.setState((state) => ({
      user: state.user ? { ...state.user, name, email } : null,
    }));
    setSavingProfile(false);
    setProfileSuccess(true);
    setTimeout(() => setProfileSuccess(false), 3000);
  };

  if (!user) return null;

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6">
      <PageHeader
        title="Profile"
        description="Manage your personal information and account settings."
      />

      {/* Identity */}
      <Card>
        <CardContent className="flex flex-wrap items-center gap-4 py-6">
          <Avatar className="size-16">
            {user.avatar && (
              <AvatarImage
                src={`/app/auth/profile/avatar?v=${avatarVersion}`}
                alt={user.name}
              />
            )}
            <AvatarFallback className="bg-primary/10 text-primary text-xl font-bold">
              {userInitials}
            </AvatarFallback>
          </Avatar>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-lg font-medium">{user.name}</span>
              {user.emailVerified ? (
                <Badge variant="secondary" className="gap-1 text-[11px]">
                  <BadgeCheck className="size-3" /> Verified
                </Badge>
              ) : (
                <Badge variant="outline" className="text-[11px]">
                  Unverified email
                </Badge>
              )}
              {user.totpEnabled ? (
                <Badge variant="secondary" className="gap-1 text-[11px]">
                  <ShieldCheck className="size-3" /> 2FA on
                </Badge>
              ) : (
                <Badge variant="outline" className="gap-1 text-[11px]">
                  <ShieldOff className="size-3" /> 2FA off
                </Badge>
              )}
            </div>
            <div className="truncate text-sm text-muted-foreground">{user.email}</div>
          </div>
          <div className="flex flex-col items-start gap-1.5">
            <input
              ref={avatarInputRef}
              type="file"
              accept="image/jpeg,image/png,image/webp"
              className="hidden"
              onChange={(e) => {
                const file = e.target.files?.[0];
                e.target.value = ""; // let the same file be re-picked
                if (file) void handleAvatarUpload(file);
              }}
            />
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                disabled={avatarBusy}
                onClick={() => avatarInputRef.current?.click()}
              >
                {avatarBusy ? "Working..." : user.avatar ? "Change" : "Upload"}
              </Button>
              {user.avatar && (
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={avatarBusy}
                  onClick={() => void handleAvatarRemove()}
                >
                  Remove
                </Button>
              )}
            </div>
            {avatarError && <span className="text-xs text-destructive">{avatarError}</span>}
          </div>
        </CardContent>
      </Card>

      {/* Projects */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2.5">
            <span className="flex size-8 items-center justify-center rounded-lg bg-info/10">
              <FolderKanban className="size-4 text-info" />
            </span>
            Projects
          </CardTitle>
          <CardDescription>
            {projects.length === 0
              ? "You are not a member of any project yet."
              : `${projects.length} project${projects.length === 1 ? "" : "s"} you're a member of. Select one to switch to it.`}
          </CardDescription>
        </CardHeader>
        <CardContent>
          {projects.length === 0 ? (
            <p className="text-sm text-muted-foreground">No projects yet.</p>
          ) : (
            <div className="space-y-2">
              {projects.map((p, i) => (
                <button
                  key={p.id}
                  type="button"
                  onClick={() => {
                    setCurrentProject(p);
                    router.push("/");
                  }}
                  className="flex w-full items-center gap-3 rounded-lg border px-3 py-2.5 text-left transition-colors hover:bg-muted/50"
                >
                  <div className={`flex size-8 shrink-0 items-center justify-center rounded-md text-xs font-bold ${PROJECT_TILE_TONES[i % PROJECT_TILE_TONES.length]}`}>
                    {initials(p.name)}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm font-medium">{p.name}</span>
                      {p.id === currentProject?.id && (
                        <Badge variant="secondary" className="text-[10px]">Current</Badge>
                      )}
                    </div>
                    {p.description && (
                      <p className="truncate text-xs text-muted-foreground">{p.description}</p>
                    )}
                  </div>
                  <ActiveBadge active={p.active} />
                </button>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Personal Info */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2.5">
            <span className="flex size-8 items-center justify-center rounded-lg bg-primary/10">
              <User className="size-4 text-primary" />
            </span>
            Personal Information
          </CardTitle>
          <CardDescription>Update your name and email address.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
              />
            </div>
          </div>
          <div className="flex items-center gap-3">
            <Button
              onClick={handleSaveProfile}
              disabled={savingProfile || (!name.trim() || !email.trim())}
            >
              {savingProfile ? "Saving..." : "Save changes"}
            </Button>
            {profileSuccess && (
              <span className="text-sm text-success">
                Profile updated successfully.
              </span>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Appearance */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2.5">
            <span className="flex size-8 items-center justify-center rounded-lg bg-violet-500/10">
              <Palette className="size-4 text-violet-600 dark:text-violet-400" />
            </span>
            Appearance
          </CardTitle>
          <CardDescription>
            Choose how the dashboard looks on this device.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-3 gap-3">
            {THEMES.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => setTheme(item.id)}
                aria-pressed={themeMounted && theme === item.id}
                className={cn(
                  "flex flex-col items-center gap-2 rounded-lg border p-4 transition-colors",
                  themeMounted && theme === item.id
                    ? "border-primary bg-primary/5"
                    : "hover:border-muted-foreground/30",
                )}
              >
                <item.icon className="size-5 text-muted-foreground" />
                <span className="text-xs font-medium">{item.label}</span>
              </button>
            ))}
          </div>
        </CardContent>
      </Card>

    </div>
  );
}
