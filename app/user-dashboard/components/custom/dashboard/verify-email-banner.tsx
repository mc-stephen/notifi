"use client";

import { useState } from "react";
import { AlertTriangle, Loader2, MailCheck } from "lucide-react";

import { Button } from "@/components/ui/button";
import { useAuthStore } from "@/store/auth-store";

type Feedback = { kind: "ok" | "error"; text: string } | null;

export function VerifyEmailBanner() {
  const user = useAuthStore((s) => s.user);
  const authHydrated = useAuthStore((s) => s.authHydrated);
  const resendVerification = useAuthStore((s) => s.resendVerification);
  const [sending, setSending] = useState(false);
  const [feedback, setFeedback] = useState<Feedback>(null);

  if (!authHydrated || !user || user.emailVerified) return null;

  const handleResend = async () => {
    setSending(true);
    setFeedback(null);

    const result = (await resendVerification(user.email)) as
      | { error?: string }
      | undefined;

    setSending(false);
    if (result?.error) {
      setFeedback({ kind: "error", text: result.error });
    } else {
      setFeedback({
        kind: "ok",
        text: "Verification email sent — check your inbox.",
      });
    }
  };

  return (
    <div className="flex flex-col gap-3 border-b border-amber-500/20 bg-amber-500/5 px-4 py-3 sm:flex-row sm:items-center sm:justify-between lg:px-6">
      <div className="flex items-start gap-3">
        <AlertTriangle className="mt-0.5 size-4 shrink-0 text-amber-600 dark:text-amber-400" />
        <div className="space-y-0.5">
          <p className="text-sm font-medium text-amber-600 dark:text-amber-400">
            Verify your email address
          </p>
          <p className="text-sm text-muted-foreground">
            We sent a verification link to{" "}
            <span className="font-medium text-foreground">{user.email}</span>.
            Your account will be deleted in 48 hours if your email is not
            verified.
          </p>
          {feedback && (
            <p
              className={
                feedback.kind === "ok"
                  ? "text-xs text-success"
                  : "text-xs text-destructive"
              }
            >
              {feedback.text}
            </p>
          )}
        </div>
      </div>

      <Button
        size="sm"
        variant="outline"
        onClick={handleResend}
        disabled={sending}
        className="shrink-0 self-start sm:self-center"
      >
        {sending ? (
          <>
            <Loader2 className="mr-2 size-3 animate-spin" />
            Sending...
          </>
        ) : (
          <>
            <MailCheck className="mr-2 size-3" />
            Resend email
          </>
        )}
      </Button>
    </div>
  );
}
