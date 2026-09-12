"use client";

import { use, useEffect, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { CheckCircle2, XCircle, MailOpen, Loader2, LogIn } from "lucide-react";
import { api, ApiError } from "@/lib/api";

type InvitePreview = {
  id: string;
  projectId: string;
  projectName: string;
  email: string;
  role: string;
  expiresAt: string;
};

type Status =
  | { kind: "loading" }
  | { kind: "needs-auth" }
  | { kind: "ready"; invite: InvitePreview }
  | { kind: "accepted"; projectName: string }
  | { kind: "declined" }
  | { kind: "error"; title: string; message: string };

function toErrorStatus(e: unknown): Status {
  const status = e instanceof ApiError ? e.status : undefined;
  const detail = e instanceof Error ? e.message : "";
  const lowered = detail.toLowerCase();
  if (status === 401) return { kind: "needs-auth" };
  if (status === 410 || lowered.includes("expired")) {
    return {
      kind: "error",
      title: "Invitation expired",
      message:
        "This invitation is older than 7 days and can no longer be accepted. Ask a project owner or admin to send you a fresh invite.",
    };
  }
  if (status === 403 && lowered.includes("two-factor")) {
    return {
      kind: "error",
      title: "Two-factor authentication required",
      message:
        "This project requires 2FA for new members. Enable two-factor authentication on your profile first, then open this link again to accept.",
    };
  }
  if (status === 403 && lowered.includes("different account")) {
    return {
      kind: "error",
      title: "Wrong account",
      message:
        "This invitation belongs to a different account. Sign in with the invited email address, then open this link again.",
    };
  }
  return {
    kind: "error",
    title: "Invitation unavailable",
    message:
      detail ||
      "This invitation link is invalid or has already been used.",
  };
}

export default function InvitePage({ params }: { params: Promise<{ token: string }> }) {
  const { token } = use(params);
  const router = useRouter();
  const [status, setStatus] = useState<Status>({ kind: "loading" });
  const [acting, setActing] = useState<"accept" | "decline" | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  useEffect(() => {
    let ignore = false;
    (async () => {
      try {
        const res = await api<{ invite: InvitePreview }>(
          `/app/invites/${encodeURIComponent(token)}`
        );
        if (!ignore) setStatus({ kind: "ready", invite: res.invite });
      } catch (e) {
        if (!ignore) setStatus(toErrorStatus(e));
      }
    })();
    return () => {
      ignore = true;
    };
  }, [token]);

  async function act(action: "accept" | "decline") {
    if (status.kind !== "ready") return;
    const projectName = status.invite.projectName;
    setActing(action);
    setActionError(null);
    try {
      await api(`/app/invites/${encodeURIComponent(token)}/${action}`, { method: "POST" });
      setStatus({ kind: action === "accept" ? "accepted" : "declined", ...(action === "accept" ? { projectName } : {}) } as Status);
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) {
        setStatus({ kind: "needs-auth" });
        return;
      }
      setActionError(e instanceof Error ? e.message : "Something went wrong.");
    } finally {
      setActing(null);
    }
  }

  const next = `/invite/${encodeURIComponent(token)}`;

  return (
    <div className="mx-auto flex min-h-[70vh] w-full max-w-xl flex-col justify-center space-y-6 py-12">
      <div className="text-center">
        <p className="text-xl font-bold tracking-tight text-primary">Notifi</p>
        <h1 className="mt-2 text-2xl font-bold tracking-tight">Team invitation</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Review and respond to your invitation.
        </p>
      </div>

      <Card>
        {status.kind === "loading" && (
          <CardContent className="space-y-3 py-8" aria-label="Loading invitation">
            <div className="h-5 w-2/3 rounded bg-muted animate-pulse" />
            <div className="h-4 w-full rounded bg-muted animate-pulse" />
            <div className="h-9 w-32 rounded bg-muted animate-pulse" />
          </CardContent>
        )}

        {status.kind === "needs-auth" && (
          <>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <LogIn className="size-5 text-muted-foreground" />
                Sign in to review
              </CardTitle>
              <CardDescription>
                You need to be signed in to view this invitation — sign in first, then come back to accept or decline.
              </CardDescription>
            </CardHeader>
            <CardContent className="flex gap-2">
              <Button size="sm" render={<Link href={`/auth/login?next=${next}`} />}>
                Sign in
              </Button>
              <Button size="sm" variant="outline" render={<Link href={`/auth/signup?next=${next}`} />}>
                Create account
              </Button>
            </CardContent>
          </>
        )}

        {status.kind === "error" && (
          <>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <XCircle className="size-5 text-destructive" />
                {status.title}
              </CardTitle>
              <CardDescription>{status.message}</CardDescription>
            </CardHeader>
            <CardContent>
              <Button size="sm" variant="outline" render={<Link href="/" />}>
                Back to dashboard
              </Button>
            </CardContent>
          </>
        )}

        {status.kind === "ready" && (
          <>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <MailOpen className="size-5 text-muted-foreground" />
                You&apos;re invited
              </CardTitle>
              <CardDescription>
                Join <span className="font-medium text-foreground">{status.invite.projectName}</span>{" "}
                as <Badge variant="secondary">{status.invite.role}</Badge>
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-xs text-muted-foreground">
                Invited as {status.invite.email} · expires{" "}
                {new Date(status.invite.expiresAt).toLocaleDateString()}.
              </p>
              {actionError && <p className="text-sm text-destructive">{actionError}</p>}
              <div className="flex gap-2">
                <Button
                  size="sm"
                  disabled={acting !== null}
                  onClick={() => act("accept")}
                >
                  {acting === "accept" ? (
                    <>
                      <Loader2 className="size-3.5 animate-spin" /> Accepting...
                    </>
                  ) : (
                    "Accept invitation"
                  )}
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={acting !== null}
                  onClick={() => act("decline")}
                >
                  {acting === "decline" ? "Declining..." : "Decline"}
                </Button>
              </div>
            </CardContent>
          </>
        )}

        {(status.kind === "accepted" || status.kind === "declined") && (
          <>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                {status.kind === "accepted" ? (
                  <>
                    <CheckCircle2 className="size-5 text-success" />
                    You&apos;re on the team
                  </>
                ) : (
                  <>
                    <XCircle className="size-5 text-muted-foreground" />
                    Invitation declined
                  </>
                )}
              </CardTitle>
              <CardDescription>
                {status.kind === "accepted"
                  ? `Welcome aboard — the project is now in your workspace.`
                  : "No problem — nothing was changed on your account."}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Button size="sm" onClick={() => router.push("/")}>
                Go to dashboard
              </Button>
            </CardContent>
          </>
        )}
      </Card>
    </div>
  );
}
