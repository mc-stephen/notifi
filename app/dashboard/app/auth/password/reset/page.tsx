"use client";

import * as z from "zod";
import Link from "next/link";
import { Suspense, useState } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useRouter, useSearchParams } from "next/navigation";
import { Loader2, CheckCircle2, ArrowLeft } from "lucide-react";
import {
  useForm,
  useWatch,
  UseFormRegister,
  FieldErrors,
} from "react-hook-form";

import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { AuthHeaderMobile } from "@/components/custom/auth/auth-header";
import { PasswordInput } from "@/components/custom/auth/password-input";
import { PasswordStrength } from "@/components/custom/auth/password-strength";

import { useAuth } from "@/hooks/use-auth";

//================================
// Schema definition with cross-field validation
//================================
const resetPasswordSchema = z
  .object({
    password: z
      .string()
      .min(1, "Password is required")
      .min(8, "Password must be at least 8 characters"),
    confirmPassword: z.string().min(1, "Please confirm your password"),
  })
  .refine((data) => data.password === data.confirmPassword, {
    message: "Passwords do not match",
    path: ["confirmPassword"],
  });

type ResetPasswordFormValues = z.infer<typeof resetPasswordSchema>;

export default function ResetPasswordPage() {
  return (
    <Suspense fallback={<LoadingFallback />}>
      <ResetPasswordForm />
    </Suspense>
  );
}

//================================
//
//================================
function ResetPasswordForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const token = searchParams.get("token") || "";
  const { resetPassword, isLoading } = useAuth();

  const [serverError, setServerError] = useState<string | null>(null);
  const [isSuccess, setIsSuccess] = useState(false);

  const {
    register,
    handleSubmit,
    control,
    formState: { errors },
  } = useForm<ResetPasswordFormValues>({
    resolver: zodResolver(resetPasswordSchema),
    defaultValues: {
      password: "",
      confirmPassword: "",
    },
  });

  const passwordValue = useWatch({
    control,
    name: "password",
    defaultValue: "",
  });

  const onSubmit = async (data: ResetPasswordFormValues) => {
    setServerError(null);

    if (!token) {
      setServerError("Reset token is missing or invalid.");
      return;
    }

    const result = (await resetPassword(token, data.password)) as
      { error?: string } | undefined;

    if (result?.error) {
      setServerError(result.error);
    } else {
      setIsSuccess(true);
      setTimeout(() => {
        router.push("/auth/login");
      }, 2000);
    }
  };

  return (
    <>
      <AuthHeaderMobile />
      {isSuccess ? (
        <SuccessState />
      ) : (
        <FormState
          register={register}
          onSubmit={handleSubmit(onSubmit)}
          isLoading={isLoading}
          serverError={serverError}
          errors={errors}
          passwordValue={passwordValue}
        />
      )}
    </>
  );
}

//================================
//
//================================
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

//================================
//
//================================
function SuccessState() {
  return (
    <div className="space-y-6">
      <div className="flex flex-col items-center text-center">
        <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-success/10">
          <CheckCircle2 className="size-6 text-success" />
        </div>
        <h1 className="text-2xl font-bold tracking-tight">Password updated!</h1>
        <p className="mt-2 text-muted-foreground">
          Your password has been successfully reset. Redirecting to sign in...
        </p>
      </div>
    </div>
  );
}

//================================
//
//================================
function FormState({
  register,
  onSubmit,
  isLoading,
  serverError,
  errors,
  passwordValue,
}: {
  register: UseFormRegister<ResetPasswordFormValues>;
  onSubmit: (e?: React.BaseSyntheticEvent) => Promise<void>;
  isLoading: boolean;
  serverError: string | null;
  errors: FieldErrors<ResetPasswordFormValues>;
  passwordValue: string;
}) {
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h1 className="text-2xl font-bold tracking-tight">
          Create new password
        </h1>
        <p className="text-muted-foreground">
          Your new password must be different from previously used passwords
        </p>
      </div>

      {serverError && <ErrorBanner message={serverError} />}

      <form onSubmit={onSubmit} className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="password">New password</Label>
          <PasswordInput
            id="password"
            placeholder="Enter your new password"
            {...register("password")}
            error={errors.password?.message}
            aria-invalid={!!errors.password}
            aria-describedby={errors.password ? "password-error" : undefined}
          />
          {errors.password && (
            <p id="password-error" className="text-xs text-destructive">
              {errors.password.message}
            </p>
          )}
          <PasswordStrength password={passwordValue} />
        </div>

        <div className="space-y-2">
          <Label htmlFor="confirmPassword">Confirm new password</Label>
          <PasswordInput
            id="confirmPassword"
            placeholder="Confirm your new password"
            {...register("confirmPassword")}
            error={errors.confirmPassword?.message}
            aria-invalid={!!errors.confirmPassword}
            aria-describedby={
              errors.confirmPassword ? "confirm-password-error" : undefined
            }
          />
          {errors.confirmPassword && (
            <p id="confirm-password-error" className="text-xs text-destructive">
              {errors.confirmPassword.message}
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
