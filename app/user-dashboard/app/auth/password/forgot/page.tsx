"use client";

import * as z from "zod";
import Link from "next/link";
import { useState } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { Loader2, CheckCircle2, ArrowLeft, Mail } from "lucide-react";
import { useForm, UseFormRegister, FieldErrors } from "react-hook-form";

import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { AuthInput } from "@/components/custom/auth/auth-input";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { AuthHeaderMobile } from "@/components/custom/auth/auth-header";

import { useAuth } from "@/hooks/use-auth";

const forgotPasswordSchema = z.object({
  email: z
    .string()
    .min(1, "Email is required")
    .email("Please enter a valid email address"),
});

type ForgotPasswordFormValues = z.infer<typeof forgotPasswordSchema>;

export default function ForgotPasswordPage() {
  const { forgotPassword, isLoading } = useAuth();
  const [serverError, setServerError] = useState<string | null>(null);
  const [submittedEmail, setSubmittedEmail] = useState<string | null>(null);

  const {
    register,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<ForgotPasswordFormValues>({
    resolver: zodResolver(forgotPasswordSchema),
    defaultValues: { email: "" },
  });

  const onSubmit = async (data: ForgotPasswordFormValues) => {
    setServerError(null);
    const result = (await forgotPassword(data.email)) as
      { error?: string } | undefined;

    if (result?.error) {
      setServerError(result.error);
    } else {
      setSubmittedEmail(data.email);
    }
  };

  const handleResetForm = () => {
    setSubmittedEmail(null);
    setServerError(null);
    reset();
  };

  return (
    <>
      <AuthHeaderMobile />
      {submittedEmail ? (
        <SuccessState
          email={submittedEmail}
          onTryDifferentEmail={handleResetForm}
        />
      ) : (
        <FormState
          errors={errors}
          register={register}
          isLoading={isLoading}
          serverError={serverError}
          onSubmit={handleSubmit(onSubmit)}
        />
      )}
    </>
  );
}

//================================
// Sub-views (Defined outside main component)
//================================
function SuccessState({
  email,
  onTryDifferentEmail,
}: {
  email: string;
  onTryDifferentEmail: () => void;
}) {
  return (
    <div className="space-y-6">
      <div className="flex flex-col items-center text-center">
        <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-success/10">
          <CheckCircle2 className="size-6 text-success" />
        </div>
        <h1 className="text-2xl font-bold tracking-tight">Check your email</h1>
        <p className="mt-2 text-muted-foreground">
          We sent a password reset link to{" "}
          <span className="font-medium text-foreground">{email}</span>
        </p>
      </div>

      <div className="rounded-lg border bg-muted/50 p-4 text-sm text-muted-foreground">
        <p>
          Didn&apos;t receive the email? Check your spam folder, or{" "}
          <button
            type="button"
            onClick={onTryDifferentEmail}
            className="font-medium text-primary hover:underline"
          >
            try a different email
          </button>
        </p>
      </div>

      <Button variant="ghost" className="h-10 w-full">
        <Link
          href="/auth/login"
          className="flex items-center justify-center gap-2"
        >
          <ArrowLeft className="size-4 shrink-0" />
          <span>Back to sign in</span>
        </Link>
      </Button>
    </div>
  );
}

//================================
// Sub-views (Defined outside main component)
//================================
function FormState({
  register,
  onSubmit,
  isLoading,
  serverError,
  errors,
}: {
  register: UseFormRegister<ForgotPasswordFormValues>;
  onSubmit: (e?: React.BaseSyntheticEvent) => Promise<void>;
  isLoading: boolean;
  serverError: string | null;
  errors: FieldErrors<ForgotPasswordFormValues>;
}) {
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h1 className="text-2xl font-bold tracking-tight">
          Forgot your password?
        </h1>
        <p className="text-muted-foreground">
          Enter your email and we&apos;ll send you a reset link
        </p>
      </div>

      {serverError && <ErrorBanner message={serverError} />}

      <form onSubmit={onSubmit} className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="email">Email</Label>
          <AuthInput
            id="email"
            type="email"
            icon={Mail}
            placeholder="Enter your email"
            {...register("email")}
            aria-invalid={!!errors.email}
            aria-describedby={errors.email ? "email-error" : undefined}
          />
          {errors.email && (
            <p id="email-error" className="text-xs text-destructive">
              {errors.email.message}
            </p>
          )}
        </div>

        <Button type="submit" className="h-10 w-full" disabled={isLoading}>
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

      <Button variant="ghost" className="h-10 w-full">
        <Link
          href="/auth/login"
          className="flex items-center justify-center gap-2"
        >
          <ArrowLeft className="size-4 shrink-0" />
          <span>Back to sign in</span>
        </Link>
      </Button>
    </div>
  );
}
