"use client";

import { Suspense, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";
import { Loader2, CheckCircle2, ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { AuthHeaderMobile } from "@/components/custom/auth/auth-header";
import { PasswordInput } from "@/components/custom/auth/password-input";
import { PasswordStrength } from "@/components/custom/auth/password-strength";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { useAuth } from "@/hooks/use-auth";

function ResetPasswordForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const token = searchParams.get("token") || "";
  const { resetPassword, isLoading } = useAuth();

  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<{
    password?: string;
    confirmPassword?: string;
  }>({});
  const [success, setSuccess] = useState(false);

  const validate = () => {
    const errors: typeof fieldErrors = {};

    if (!password) {
      errors.password = "Password is required";
    } else if (password.length < 8) {
      errors.password = "Password must be at least 8 characters";
    }

    if (!confirmPassword) {
      errors.confirmPassword = "Please confirm your password";
    } else if (password !== confirmPassword) {
      errors.confirmPassword = "Passwords do not match";
    }

    setFieldErrors(errors);
    return Object.keys(errors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!validate()) return;

    const result = await resetPassword(token, password);

    if (result.error) {
      setError(result.error);
    } else {
      setSuccess(true);
      setTimeout(() => {
        router.push("/auth/login");
      }, 2000);
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
              Password updated!
            </h1>
            <p className="mt-2 text-muted-foreground">
              Your password has been successfully reset. Redirecting to sign
              in...
            </p>
          </div>
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
            Create new password
          </h1>
          <p className="text-muted-foreground">
            Your new password must be different from previously used passwords
          </p>
        </div>

        {error && <ErrorBanner message={error} />}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="password">New password</Label>
            <PasswordInput
              id="password"
              placeholder="Enter your new password"
              value={password}
              onChange={(e) => {
                setPassword(e.target.value);
                setFieldErrors((prev) => ({ ...prev, password: undefined }));
              }}
              error={fieldErrors.password}
              aria-invalid={!!fieldErrors.password}
              aria-describedby={
                fieldErrors.password ? "password-error" : undefined
              }
            />
            {fieldErrors.password && (
              <p id="password-error" className="text-xs text-destructive">
                {fieldErrors.password}
              </p>
            )}
            <PasswordStrength password={password} />
          </div>

          <div className="space-y-2">
            <Label htmlFor="confirmPassword">Confirm new password</Label>
            <PasswordInput
              id="confirmPassword"
              placeholder="Confirm your new password"
              value={confirmPassword}
              onChange={(e) => {
                setConfirmPassword(e.target.value);
                setFieldErrors((prev) => ({
                  ...prev,
                  confirmPassword: undefined,
                }));
              }}
              error={fieldErrors.confirmPassword}
              aria-invalid={!!fieldErrors.confirmPassword}
              aria-describedby={
                fieldErrors.confirmPassword
                  ? "confirm-password-error"
                  : undefined
              }
            />
            {fieldErrors.confirmPassword && (
              <p
                id="confirm-password-error"
                className="text-xs text-destructive"
              >
                {fieldErrors.confirmPassword}
              </p>
            )}
          </div>

          <Button type="submit" className="h-10 w-full" disabled={isLoading}>
            {isLoading ? (
              <>
                <Loader2 className="mr-2 size-4 animate-spin" />
                Resetting password...
              </>
            ) : (
              "Reset password"
            )}
          </Button>
        </form>

        <Link href="/auth/login">
          <Button variant="ghost" className="h-10 w-full gap-2">
            <ArrowLeft className="size-4" />
            Back to sign in
          </Button>
        </Link>
      </div>
    </>
  );
}

export default function ResetPasswordPage() {
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
      <ResetPasswordForm />
    </Suspense>
  );
}
