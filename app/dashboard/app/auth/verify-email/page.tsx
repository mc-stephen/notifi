"use client";

import { Suspense, useState, useEffect } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import {
  Loader2,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Mail,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { AuthHeaderMobile } from "@/components/custom/auth/auth-header";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { useAuth } from "@/hooks/use-auth";

type VerificationState = "loading" | "success" | "expired" | "invalid" | "resent";

function VerifyEmailForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const token = searchParams.get("token") || "";
  const { verifyEmail, isLoading } = useAuth();

  const [state, setState] = useState<VerificationState>("loading");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!token) {
      setState("resent");
      return;
    }

    const verify = async () => {
      const result = await verifyEmail(token);

      if (result.error) {
        if (result.error.includes("expired")) {
          setState("expired");
        } else {
          setState("invalid");
        }
      } else {
        setState("success");
        setTimeout(() => {
          router.push("/onboarding/welcome");
        }, 3000);
      }
    };

    verify();
  }, [token, verifyEmail, router]);

  const handleResend = async () => {
    setError(null);
    // Mock resend - in real app would call API
    await new Promise((resolve) => setTimeout(resolve, 1000));
    setState("resent");
  };

  const handleBackToLogin = () => {
    router.push("/auth/login");
  };

  return (
    <>
      <AuthHeaderMobile />

      <div className="space-y-6">
        {state === "loading" && (
          <div className="flex flex-col items-center text-center">
            <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-muted">
              <Loader2 className="size-6 animate-spin text-muted-foreground" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight">
              Verifying your email...
            </h1>
            <p className="mt-2 text-muted-foreground">
              Please wait while we verify your email address.
            </p>
          </div>
        )}

        {state === "success" && (
          <div className="flex flex-col items-center text-center">
            <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-success/10">
              <CheckCircle2 className="size-6 text-success" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight">
              Email verified!
            </h1>
            <p className="mt-2 text-muted-foreground">
              Your account has been verified. Redirecting to dashboard...
            </p>
          </div>
        )}

        {state === "expired" && (
          <div className="flex flex-col items-center text-center">
            <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-warning/10">
              <AlertTriangle className="size-6 text-warning" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight">Link expired</h1>
            <p className="mt-2 text-muted-foreground">
              This verification link has expired. Please request a new one.
            </p>

            {error && <ErrorBanner message={error} className="mt-4" />}

            <div className="mt-6 flex flex-col gap-3">
              <Button onClick={handleResend} disabled={isLoading}>
                {isLoading ? (
                  <>
                    <Loader2 className="mr-2 size-4 animate-spin" />
                    Sending...
                  </>
                ) : (
                  <>
                    <Mail className="mr-2 size-4" />
                    Resend verification email
                  </>
                )}
              </Button>
              <Button variant="ghost" onClick={handleBackToLogin}>
                Back to sign in
              </Button>
            </div>
          </div>
        )}

        {state === "invalid" && (
          <div className="flex flex-col items-center text-center">
            <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-destructive/10">
              <XCircle className="size-6 text-destructive" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight">Invalid link</h1>
            <p className="mt-2 text-muted-foreground">
              This verification link is invalid. Please request a new one.
            </p>

            {error && <ErrorBanner message={error} className="mt-4" />}

            <div className="mt-6 flex flex-col gap-3">
              <Button onClick={handleResend} disabled={isLoading}>
                {isLoading ? (
                  <>
                    <Loader2 className="mr-2 size-4 animate-spin" />
                    Sending...
                  </>
                ) : (
                  <>
                    <Mail className="mr-2 size-4" />
                    Resend verification email
                  </>
                )}
              </Button>
              <Button variant="ghost" onClick={handleBackToLogin}>
                Back to sign in
              </Button>
            </div>
          </div>
        )}

        {state === "resent" && (
          <div className="flex flex-col items-center text-center">
            <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-success/10">
              <CheckCircle2 className="size-6 text-success" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight">
              Verification email sent
            </h1>
            <p className="mt-2 text-muted-foreground">
              Check your inbox for a new verification link.
            </p>

            <div className="mt-6 flex flex-col gap-3">
              <Button onClick={handleResend} disabled={isLoading}>
                {isLoading ? (
                  <>
                    <Loader2 className="mr-2 size-4 animate-spin" />
                    Sending...
                  </>
                ) : (
                  <>
                    <Mail className="mr-2 size-4" />
                    Resend verification email
                  </>
                )}
              </Button>
              <Button variant="ghost" onClick={handleBackToLogin}>
                Back to sign in
              </Button>
            </div>
          </div>
        )}
      </div>
    </>
  );
}

export default function VerifyEmailPage() {
  return (
    <Suspense
      fallback={
        <div className="flex flex-col items-center text-center">
          <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-muted">
            <Loader2 className="size-6 animate-spin text-muted-foreground" />
          </div>
          <h1 className="text-2xl font-bold tracking-tight">Loading...</h1>
        </div>
      }
    >
      <VerifyEmailForm />
    </Suspense>
  );
}
