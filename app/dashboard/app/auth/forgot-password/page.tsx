"use client";

import { useState } from "react";
import Link from "next/link";
import { Loader2, CheckCircle2, ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { AuthHeaderMobile } from "@/components/custom/auth/auth-header";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { useAuth } from "@/hooks/use-auth";

export default function ForgotPasswordPage() {
  const { forgotPassword, isLoading } = useAuth();

  const [email, setEmail] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const [sentEmail, setSentEmail] = useState("");

  const validate = () => {
    if (!email) {
      setError("Email is required");
      return false;
    }
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      setError("Please enter a valid email address");
      return false;
    }
    return true;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!validate()) return;

    const result = await forgotPassword(email);

    if (result.error) {
      setError(result.error);
    } else {
      setSentEmail(email);
      setSuccess(true);
    }
  };

  if (success) {
    return (
      <>
        <AuthHeaderMobile />

        <div className="space-y-6">
          <div className="flex flex-col items-center text-center">
            <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-success/10">
              <CheckCircle2 className="size-6 text-success" />
            </div>
            <h1 className="text-2xl font-bold tracking-tight">
              Check your email
            </h1>
            <p className="mt-2 text-muted-foreground">
              We sent a password reset link to{" "}
              <span className="font-medium text-foreground">{sentEmail}</span>
            </p>
          </div>

          <div className="rounded-lg border bg-muted/50 p-4 text-sm text-muted-foreground">
            <p>
              Didn&apos;t receive the email? Check your spam folder, or{" "}
              <button
                type="button"
                onClick={() => {
                  setSuccess(false);
                  setEmail("");
                }}
                className="font-medium text-primary hover:underline"
              >
                try a different email
              </button>
            </p>
          </div>

          <Link href="/auth/login">
            <Button variant="ghost" className="w-full gap-2">
              <ArrowLeft className="size-4" />
              Back to sign in
            </Button>
          </Link>
        </div>
      </>
    );
  }

  return (
    <>
      <AuthHeaderMobile />

      <div className="space-y-6">
        <div className="space-y-2">
          <h1 className="text-2xl font-bold tracking-tight">
            Forgot your password?
          </h1>
          <p className="text-muted-foreground">
            Enter your email and we&apos;ll send you a reset link
          </p>
        </div>

        {error && <ErrorBanner message={error} />}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="email">Email</Label>
            <Input
              id="email"
              type="email"
              placeholder="you@example.com"
              value={email}
              onChange={(e) => {
                setEmail(e.target.value);
                setError(null);
              }}
              aria-invalid={!!error}
            />
          </div>

          <Button type="submit" className="w-full" disabled={isLoading}>
            {isLoading ? (
              <>
                <Loader2 className="mr-2 size-4 animate-spin" />
                Sending reset link...
              </>
            ) : (
              "Send reset link"
            )}
          </Button>
        </form>

        <Link href="/auth/login">
          <Button variant="ghost" className="w-full gap-2">
            <ArrowLeft className="size-4" />
            Back to sign in
          </Button>
        </Link>
      </div>
    </>
  );
}
