"use client";

import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useState, useEffect, ElementType } from "react";
import {
  Loader2,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Mail,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { AuthHeaderMobile } from "@/components/custom/auth/auth-header";

import { useAuth } from "@/hooks/use-auth";

type VerificationState =
  "loading" | "success" | "expired" | "invalid" | "resent";

interface StateConfig {
  icon: ElementType;
  iconBgClass: string;
  iconColorClass: string;
  title: string;
  description: string;
  showActions?: boolean;
}

const STATE_CONFIG: Record<VerificationState, StateConfig> = {
  loading: {
    icon: Loader2,
    iconBgClass: "bg-muted",
    iconColorClass: "text-muted-foreground animate-spin",
    title: "Verifying your email...",
    description: "Please wait while we verify your email address.",
  },
  success: {
    icon: CheckCircle2,
    iconBgClass: "bg-success/10",
    iconColorClass: "text-success",
    title: "Email verified!",
    description: "Your account has been verified. Redirecting to onboarding...",
  },
  expired: {
    icon: AlertTriangle,
    iconBgClass: "bg-warning/10",
    iconColorClass: "text-warning",
    title: "Link expired",
    description:
      "This verification link has expired. Please request a new one.",
    showActions: true,
  },
  invalid: {
    icon: XCircle,
    iconBgClass: "bg-destructive/10",
    iconColorClass: "text-destructive",
    title: "Invalid link",
    description: "This verification link is invalid. Please request a new one.",
    showActions: true,
  },
  resent: {
    icon: CheckCircle2,
    iconBgClass: "bg-success/10",
    iconColorClass: "text-success",
    title: "Verification email sent",
    description: "Check your inbox for a new verification link.",
    showActions: true,
  },
};

//============================
//
//============================
export default function VerifyEmailPage() {
  return (
    <Suspense fallback={<LoadingFallback />}>
      <VerifyEmailForm />
    </Suspense>
  );
}

//============================
//
//============================
function VerifyEmailForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const token = searchParams.get("token") || "";
  const email = searchParams.get("email") || "";
  const { verifyEmail, resendVerification, isLoading } = useAuth();

  const [state, setState] = useState<VerificationState>(
    token ? "loading" : "resent",
  );
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!token) return;

    let redirectTimer: NodeJS.Timeout;

    const verify = async () => {
      const result = (await verifyEmail(token)) as
        { error?: string } | undefined;

      if (result?.error) {
        if (result.error.includes("expired")) {
          setState("expired");
        } else {
          setState("invalid");
        }
      } else {
        setState("success");
        redirectTimer = setTimeout(() => {
          router.push("/onboarding/welcome");
        }, 3000);
      }
    };

    verify();

    return () => {
      if (redirectTimer) clearTimeout(redirectTimer);
    };
  }, [token, verifyEmail, router]);

  const handleResend = async () => {
    setError(null);
    if (!email) {
      // Direct visits (e.g. from a real email link) carry no email; the API
      // resend endpoint requires one. Point the user back to sign-in instead.
      setError(
        "We can't tell which address to resend to. Sign in and request a new verification link."
      );
      return;
    }

    const result = (await resendVerification(email)) as
      | { error?: string }
      | undefined;
    if (result?.error) {
      setError(result.error);
      return;
    }
    setState("resent");
  };

  const currentConfig = STATE_CONFIG[state];
  const Icon = currentConfig.icon;

  return (
    <>
      <AuthHeaderMobile />

      <div className="space-y-6">
        <div className="flex flex-col items-center text-center">
          <div
            className={`mb-4 flex size-12 items-center justify-center rounded-full ${currentConfig.iconBgClass}`}
          >
            <Icon className={`size-6 ${currentConfig.iconColorClass}`} />
          </div>

          <h1 className="text-2xl font-bold tracking-tight">
            {currentConfig.title}
          </h1>

          <p className="mt-2 text-muted-foreground">
            {currentConfig.description}
          </p>

          {error && <ErrorBanner message={error} className="mt-4" />}

          {currentConfig.showActions && (
            <div className="mt-6 flex w-full flex-col gap-3">
              <Button
                className="h-10 w-full"
                onClick={handleResend}
                disabled={isLoading}
              >
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

              <Button variant="ghost" className="h-10 w-full">
                <Link
                  href="/auth/login"
                  className="flex items-center justify-center gap-2"
                >
                  <span>Back to sign in</span>
                </Link>
              </Button>
            </div>
          )}
        </div>
      </div>
    </>
  );
}

//============================
//
//============================
function LoadingFallback() {
  return (
    <div className="flex flex-col items-center text-center">
      <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-muted">
        <Loader2 className="size-6 animate-spin text-muted-foreground" />
      </div>
      <h1 className="text-2xl font-bold tracking-tight">Loading...</h1>
    </div>
  );
}
