"use client";

import * as z from "zod";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { Loader2, Mail, User } from "lucide-react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm, useWatch, Controller } from "react-hook-form";

import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Separator } from "@/components/ui/separator";
import { AuthInput } from "@/components/custom/auth/auth-input";
import { AuthFooter } from "@/components/custom/auth/auth-footer";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { AuthHeaderMobile } from "@/components/custom/auth/auth-header";
import { SocialButtons } from "@/components/custom/auth/social-buttons";
import { PasswordInput } from "@/components/custom/auth/password-input";
import { PasswordStrength } from "@/components/custom/auth/password-strength";

import { useAuth } from "@/hooks/use-auth";
import { postAuthDestination } from "@/store/auth-store";
import { links } from "@/lib/env";

//================================
// Schema definition
//================================
const signupSchema = z.object({
  name: z
    .string()
    .min(1, "Name is required")
    .min(2, "Name must be at least 2 characters"),
  email: z
    .string()
    .min(1, "Email is required")
    .email("Please enter a valid email address"),
  password: z
    .string()
    .min(1, "Password is required")
    .min(8, "Password must be at least 8 characters"),
  agreedToTerms: z.boolean().refine((val) => val === true, {
    message: "You must agree to the terms",
  }),
});

type SignupFormValues = z.infer<typeof signupSchema>;

export default function SignupPage() {
  const router = useRouter();
  const { signup, loginWithOAuth, isLoading } = useAuth();
  const [serverError, setServerError] = useState<string | null>(null);

  const {
    register,
    handleSubmit,
    control,
    formState: { errors },
  } = useForm<SignupFormValues>({
    resolver: zodResolver(signupSchema),
    defaultValues: {
      name: "",
      email: "",
      password: "",
      agreedToTerms: false,
    },
  });

  const passwordValue = useWatch({
    control,
    name: "password",
    defaultValue: "",
  });

  const onSubmit = async (data: SignupFormValues) => {
    setServerError(null);

    // Signup starts a session server-side, so we go straight to onboarding
    // (and then the dashboard). The verification link arrives by email; in
    // dev mode the raw token also comes back for manual testing.
    const result = (await signup(data.name, data.email, data.password)) as
      | { error?: string }
      | undefined;

    if (result?.error) {
      setServerError(result.error);
      return;
    }

    router.push("/onboarding/welcome");
  };

  const handleGitHub = async () => {
    setServerError(null);
    const result = (await loginWithOAuth("github")) as
      | { error?: string }
      | undefined;
    if (result?.error) {
      setServerError(result.error);
      return;
    }
    router.push(postAuthDestination());
  };

  const handleGoogle = async () => {
    setServerError(null);
    const result = (await loginWithOAuth("google")) as
      | { error?: string }
      | undefined;
    if (result?.error) {
      setServerError(result.error);
      return;
    }
    router.push(postAuthDestination());
  };

  return (
    <>
      <AuthHeaderMobile />

      <div className="space-y-6">
        <div className="space-y-2">
          <h1 className="text-2xl font-bold tracking-tight">
            Create your account
          </h1>
          <p className="text-muted-foreground">
            Start sending notifications in minutes
          </p>
        </div>

        {serverError && <ErrorBanner message={serverError} />}

        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="name">Full name</Label>
            <AuthInput
              id="name"
              type="text"
              icon={User}
              placeholder="John Doe"
              {...register("name")}
              aria-invalid={!!errors.name}
              aria-describedby={errors.name ? "name-error" : undefined}
            />
            {errors.name && (
              <p id="name-error" className="text-xs text-destructive">
                {errors.name.message}
              </p>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="email">Email</Label>
            <AuthInput
              id="email"
              type="email"
              icon={Mail}
              placeholder="you@example.com"
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

          <div className="space-y-2">
            <Label htmlFor="password">Password</Label>
            <PasswordInput
              id="password"
              placeholder="Enter your password"
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
            <div className="flex items-start gap-2">
              <Controller
                control={control}
                name="agreedToTerms"
                render={({ field }) => (
                  <Checkbox
                    id="terms"
                    checked={field.value}
                    onCheckedChange={field.onChange}
                    aria-invalid={!!errors.agreedToTerms}
                    aria-describedby={
                      errors.agreedToTerms ? "terms-error" : undefined
                    }
                    className="mt-0.5"
                  />
                )}
              />
              <Label
                htmlFor="terms"
                className="text-sm font-normal leading-snug"
              >
                I agree to the{" "}
                <a
                  href={links.terms}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-primary hover:underline"
                >
                  Terms of Service
                </a>{" "}
                and{" "}
                <a
                  href={links.privacy}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-primary hover:underline"
                >
                  Privacy Policy
                </a>
              </Label>
            </div>
            {errors.agreedToTerms && (
              <p id="terms-error" className="text-xs text-destructive">
                {errors.agreedToTerms.message}
              </p>
            )}
          </div>

          <Button type="submit" className="h-10 w-full" disabled={isLoading}>
            {isLoading ? (
              <>
                <Loader2 className="mr-2 size-4 animate-spin" />
                Creating account...
              </>
            ) : (
              "Create account"
            )}
          </Button>
        </form>

        <div className="relative">
          <div className="absolute inset-0 flex items-center">
            <Separator className="w-full" />
          </div>
          <div className="relative flex justify-center text-xs uppercase">
            <span className="bg-background px-2 text-muted-foreground">
              or continue with
            </span>
          </div>
        </div>

        <SocialButtons
          onGitHub={handleGitHub}
          onGoogle={handleGoogle}
          isLoading={isLoading}
        />

        <AuthFooter mode="signup" />
      </div>
    </>
  );
}

// Real flow (wired to the Rust API):
// - Signup POSTs /v1/auth/signup, which creates the account AND starts a
//   short-lived session (rememberMe=false) — so this continues straight to
//   onboarding and then the dashboard.
// - The backend emails a welcome mail plus a verification link; that link is
//   the only path to /auth/verify-email?token=... In dev mode (no email
//   delivery yet) the API also returns a raw verificationToken for manual
//   testing of that page.
// - Until verified, the dashboard shows an unverified-email banner warning
//   that unverified accounts are deleted after 48 hours.
// - OAuth routes are stubbed server-side (501) until provider apps exist;
//   their error text is surfaced in the banner.
