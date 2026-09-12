"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import {
  User,
  Trash2,
  AlertTriangle,
  ShieldCheck,
  ShieldOff,
  KeyRound,
  BadgeCheck,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { PageHeader } from "@/components/custom/page-header";
import { PasswordInput } from "@/components/custom/auth/password-input";
import { PasswordStrength } from "@/components/custom/auth/password-strength";
import { useAuthStore } from "@/store/auth-store";
import { renderQrCode } from "@/lib/qrcode";

export default function ProfilePage() {
  const router = useRouter();
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);
  const { totpSetup, totpVerify, totpDisable } = useAuthStore();

  // Personal info
  const [name, setName] = useState(user?.name ?? "");
  const [email, setEmail] = useState(user?.email ?? "");
  const [savingProfile, setSavingProfile] = useState(false);
  const [profileSuccess, setProfileSuccess] = useState(false);

  // Password
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [savingPassword, setSavingPassword] = useState(false);
  const [passwordError, setPasswordError] = useState("");
  const [passwordSuccess, setPasswordSuccess] = useState(false);

  // Two-factor setup / disable
  const [totpSecret, setTotpSecret] = useState<string | null>(null);
  const [totpUri, setTotpUri] = useState<string | null>(null);
  const [totpCode, setTotpCode] = useState("");
  const [totpError, setTotpError] = useState<string | null>(null);
  const [totpBusy, setTotpBusy] = useState(false);
  const [showDisable, setShowDisable] = useState(false);
  const [disablePassword, setDisablePassword] = useState("");
  const [disableCode, setDisableCode] = useState("");

  // Sessions signed in to this account (mock).
  const [sessions, setSessions] = useState([
    { id: "ses_1", device: "MacBook Pro · Chrome", location: "Lagos, Nigeria", lastActive: "Active now", current: true },
    { id: "ses_2", device: "iPhone 15 · Safari", location: "Lagos, Nigeria", lastActive: "2 hours ago", current: false },
    { id: "ses_3", device: "Windows 11 · Edge", location: "London, UK", lastActive: "3 days ago", current: false },
  ]);
  const qrRef = useRef<HTMLDivElement>(null);

  // Delete account
  const [deleteEmail, setDeleteEmail] = useState("");
  const [deleting, setDeleting] = useState(false);

  const initials = user?.name
    ? user.name
        .split(" ")
        .map((n) => n[0])
        .join("")
        .toUpperCase()
        .slice(0, 2)
    : "?";

  useEffect(() => {
    if (totpUri && qrRef.current) {
      renderQrCode(qrRef.current, totpUri).catch(() =>
        setTotpError("Could not render the QR code. Use the manual secret instead.")
      );
    }
  }, [totpUri]);

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

  const handleChangePassword = async () => {
    setPasswordError("");
    setPasswordSuccess(false);

    if (!currentPassword) {
      setPasswordError("Current password is required.");
      return;
    }
    if (newPassword.length < 8) {
      setPasswordError("New password must be at least 8 characters.");
      return;
    }
    if (newPassword !== confirmPassword) {
      setPasswordError("Passwords do not match.");
      return;
    }

    setSavingPassword(true);
    await new Promise((r) => setTimeout(r, 800));
    // Mock update — in production this would call an API
    setSavingPassword(false);
    setPasswordSuccess(true);
    setCurrentPassword("");
    setNewPassword("");
    setConfirmPassword("");
    setTimeout(() => setPasswordSuccess(false), 3000);
  };

  const handleStartTotpSetup = async () => {
    setTotpError(null);
    setTotpBusy(true);
    const result = await totpSetup();
    setTotpBusy(false);
    if ("error" in result) {
      setTotpError(result.error);
      return;
    }
    setTotpSecret(result.totpSecret);
    setTotpUri(result.totpUri);
    setTotpCode("");
  };

  const handleVerifyTotp = async () => {
    setTotpError(null);
    setTotpBusy(true);
    const result = await totpVerify(totpCode);
    setTotpBusy(false);
    if (result?.error) {
      setTotpError(result.error);
      return;
    }
    setTotpSecret(null);
    setTotpUri(null);
    setTotpCode("");
  };

  const handleDisableTotp = async () => {
    setTotpError(null);
    setTotpBusy(true);
    const result = await totpDisable(disablePassword || null, disableCode);
    setTotpBusy(false);
    if (result?.error) {
      setTotpError(result.error);
      return;
    }
    setShowDisable(false);
    setDisablePassword("");
    setDisableCode("");
  };

  const handleDeleteAccount = async () => {
    if (deleteEmail !== user?.email) return;
    setDeleting(true);
    await new Promise((r) => setTimeout(r, 1000));
    // Await the API call so the httpOnly session cookie is actually
    // cleared before navigating (the proxy bounces cookie-holders).
    await logout();
    router.push("/auth/login");
  };

  if (!user) return null;

  return (
    <div className="space-y-6">
      <PageHeader
        title="Profile"
        description="Manage your personal information and account settings."
      />

      {/* Identity */}
      <Card>
        <CardContent className="flex flex-wrap items-center gap-4 py-6">
          <Avatar className="size-16">
            <AvatarFallback className="bg-primary/10 text-primary text-xl font-bold">
              {initials}
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
            <div className="text-sm text-muted-foreground">{user.email}</div>
          </div>
        </CardContent>
      </Card>

      {/* Personal Info */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <User className="size-4" />
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
              <span className="text-sm text-green-600 dark:text-green-400">
                Profile updated successfully.
              </span>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Security */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <KeyRound className="size-4" />
            Security
          </CardTitle>
          <CardDescription>
            Password and two-factor authentication for your account.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Two-factor authentication */}
          <div className="space-y-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h4 className="text-sm font-medium">Two-factor authentication</h4>
                <p className="text-sm text-muted-foreground">
                  {user.totpEnabled
                    ? "Your account requires an authenticator code at sign-in."
                    : "Add an authenticator code at sign-in. Some projects require this for team members."}
                </p>
              </div>
              {user.totpEnabled ? (
                <Badge variant="secondary" className="gap-1">
                  <ShieldCheck className="size-3" /> Enabled
                </Badge>
              ) : (
                totpUri === null && (
                  <Button size="sm" onClick={handleStartTotpSetup} disabled={totpBusy}>
                    {totpBusy ? "Starting..." : "Set up 2FA"}
                  </Button>
                )
              )}
            </div>

            {totpError && <p className="text-sm text-destructive">{totpError}</p>}

            {/* Setup flow */}
            {!user.totpEnabled && totpUri !== null && (
              <div className="rounded-lg border p-4 space-y-4">
                <ol className="space-y-1 text-sm text-muted-foreground list-decimal list-inside">
                  <li>Scan the code with your authenticator app (Google Authenticator, 1Password, …).</li>
                  <li>Enter the 6-digit code below to confirm.</li>
                </ol>
                <div className="flex flex-wrap gap-4">
                  <div ref={qrRef} className="shrink-0 rounded border bg-white p-1" />
                  <div className="min-w-0 flex-1 space-y-2">
                    <Label>Manual secret</Label>
                    <code className="block break-all rounded bg-muted px-2 py-1.5 font-mono text-xs">
                      {totpSecret}
                    </code>
                    <p className="text-xs text-muted-foreground">
                      Keep this secret safe — anyone with it can generate your codes.
                    </p>
                  </div>
                </div>
                <div className="flex flex-wrap items-end gap-2">
                  <div className="space-y-2">
                    <Label htmlFor="totp-code">Authentication code</Label>
                    <Input
                      id="totp-code"
                      inputMode="numeric"
                      autoComplete="one-time-code"
                      placeholder="123456"
                      className="w-40"
                      value={totpCode}
                      onChange={(e) => setTotpCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                    />
                  </div>
                  <Button
                    size="sm"
                    onClick={handleVerifyTotp}
                    disabled={totpBusy || totpCode.length !== 6}
                  >
                    {totpBusy ? "Verifying..." : "Verify & enable"}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setTotpUri(null);
                      setTotpSecret(null);
                      setTotpCode("");
                      setTotpError(null);
                    }}
                  >
                    Cancel
                  </Button>
                </div>
              </div>
            )}

            {/* Disable flow */}
            {user.totpEnabled && !showDisable && (
              <Button
                size="sm"
                variant="outline"
                onClick={() => {
                  setShowDisable(true);
                  setTotpError(null);
                }}
              >
                Disable 2FA
              </Button>
            )}
            {user.totpEnabled && showDisable && (
              <div className="rounded-lg border p-4 space-y-4">
                <p className="text-sm text-muted-foreground">
                  Confirm it&apos;s you: enter your current password and a fresh authenticator code.
                </p>
                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="disable-password">Current password</Label>
                    <PasswordInput
                      id="disable-password"
                      placeholder="Enter current password"
                      value={disablePassword}
                      onChange={(e) => setDisablePassword(e.target.value)}
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="disable-code">Authentication code</Label>
                    <Input
                      id="disable-code"
                      inputMode="numeric"
                      autoComplete="one-time-code"
                      placeholder="123456"
                      value={disableCode}
                      onChange={(e) => setDisableCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                    />
                  </div>
                </div>
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    variant="destructive"
                    onClick={handleDisableTotp}
                    disabled={totpBusy || disableCode.length !== 6}
                  >
                    {totpBusy ? "Disabling..." : "Disable 2FA"}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setShowDisable(false);
                      setDisablePassword("");
                      setDisableCode("");
                      setTotpError(null);
                    }}
                  >
                    Cancel
                  </Button>
                </div>
              </div>
            )}
          </div>

          <Separator />

          {/* Active sessions */}
          <div className="space-y-4">
            <div>
              <h4 className="text-sm font-medium">Active sessions</h4>
              <p className="text-sm text-muted-foreground">
                Devices currently signed in to your account.
              </p>
            </div>
            <div className="space-y-2">
              {sessions.map((session) => (
                <div
                  key={session.id}
                  className="flex flex-wrap items-center justify-between gap-3 rounded-lg border px-3 py-2.5"
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 text-sm font-medium">
                      {session.device}
                      {session.current && (
                        <Badge variant="secondary" className="text-[10px]">Current</Badge>
                      )}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {session.location} · {session.lastActive}
                    </div>
                  </div>
                  {!session.current && (
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-7 px-2 text-xs"
                      onClick={() => setSessions((prev) => prev.filter((s) => s.id !== session.id))}
                    >
                      Revoke
                    </Button>
                  )}
                </div>
              ))}
            </div>
          </div>

          <Separator />

          {/* Password */}
          <div className="space-y-4">
            <div>
              <h4 className="text-sm font-medium">Password</h4>
              <p className="text-sm text-muted-foreground">
                Change your password to keep your account secure.
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="current-password">Current password</Label>
              <PasswordInput
                id="current-password"
                placeholder="Enter current password"
                value={currentPassword}
                onChange={(e) => setCurrentPassword(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="new-password">New password</Label>
              <PasswordInput
                id="new-password"
                placeholder="Enter new password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
              />
              <PasswordStrength password={newPassword} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="confirm-password">Confirm new password</Label>
              <PasswordInput
                id="confirm-password"
                placeholder="Confirm new password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
              />
            </div>
            {passwordError && (
              <p className="text-sm text-destructive">{passwordError}</p>
            )}
            <div className="flex items-center gap-3">
              <Button
                onClick={handleChangePassword}
                disabled={savingPassword || !currentPassword || !newPassword || !confirmPassword}
              >
                {savingPassword ? "Updating..." : "Update password"}
              </Button>
              {passwordSuccess && (
                <span className="text-sm text-green-600 dark:text-green-400">
                  Password updated successfully.
                </span>
              )}
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Danger Zone */}
      <Card className="border-destructive/20">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-destructive">
            <AlertTriangle className="size-4" />
            Delete Account
          </CardTitle>
          <CardDescription>
            Permanently delete your account and all associated data. This action cannot be undone.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <p className="text-sm text-muted-foreground">
            To confirm, type your email address: <span className="font-medium text-foreground">{user.email}</span>
          </p>
          <Input
            placeholder="Type your email to confirm"
            value={deleteEmail}
            onChange={(e) => setDeleteEmail(e.target.value)}
            className="max-w-sm"
          />
          <Button
            variant="destructive"
            onClick={handleDeleteAccount}
            disabled={deleteEmail !== user.email || deleting}
          >
            <Trash2 className="mr-2 size-4" />
            {deleting ? "Deleting account..." : "Delete account"}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
