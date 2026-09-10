"use client";

import * as z from "zod";
import Link from "next/link";
import { Suspense, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Loader2, Mail, KeyRound } from "lucide-react";
import { useForm, useWatch } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";

import { Label } from "@/components/ui/label";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Separator } from "@/components/ui/separator";
import { AuthInput } from "@/components/custom/auth/auth-input";
import { AuthFooter } from "@/components/custom/auth/auth-footer";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { SocialButtons } from "@/components/custom/auth/social-buttons";
import { AuthHeaderMobile } from "@/components/custom/auth/auth-header";
import { PasswordInput } from "@/components/custom/auth/password-input";

import { useAuth } from "@/hooks/use-auth";
import { postAuthDestination } from "@/store/auth-store";

//================================
// Validation schema defined outside component to avoid re-creation
//================================
const loginSchema = z.object({
  email: z
    .string()
    .min(1, "Email is required")
    .email("Please enter a valid email address"),
  password: z.string().min(1, "Password is required"),
  rememberMe: z.boolean(),
});

type LoginFormValues = z.infer<typeof loginSchema>;

export default function LoginPage() {
  // useSearchParams requires a Suspense boundary during prerender.
  return (
    <Suspense fallback={null}>
      <LoginForm />
    </Suspense>
  );
}

function LoginForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { login, loginWithOAuth, totpChallenge, isLoading } = useAuth();
  const [serverError, setServerError] = useState<string | null>(null);
  // Second step for TOTP-enabled accounts. `?totp=1` comes from the
  // OAuth callback, which can't carry a session for the code step.
  const [totpEmail, setTotpEmail] = useState<string | null>(
    searchParams.get("totp") === "1" ? "" : null
  );
  const [totpCode, setTotpCode] = useState("");
  const [totpError, setTotpError] = useState<string | null>(null);
  const [totpLoading, setTotpLoading] = useState(false);

  // Set by the OAuth callback when popup/redirect sign-in fails.
  const oauthFailed = searchParams.get("error") === "oauth_failed";
  const bannerMessage = serverError ??
    (oauthFailed ? "Sign-in with that provider failed. Please try again." : null);

  const {
    register,
    handleSubmit,
    setValue,
    control,
    formState: { errors },
  } = useForm<LoginFormValues>({
    resolver: zodResolver(loginSchema),
    defaultValues: {
      email: "",
      password: "",
      rememberMe: false,
    },
  });

  const rememberMe = useWatch({
    control,
    name: "rememberMe",
    defaultValue: false,
  });

  //================================
  //
  //================================
  const onSubmit = async (data: LoginFormValues) => {
    setServerError(null);
    const result = (await login(data.email, data.password, data.rememberMe)) as
      { error?: string; requiresTotp?: boolean } | undefined;

    if (result?.error) {
      setServerError(result.error);
    } else if (result?.requiresTotp) {
      setTotpEmail(data.email);
      setTotpCode("");
      setTotpError(null);
    } else {
      router.push(postAuthDestination());
    }
  };

  const onTotpSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (totpEmail === null) return;
    setTotpError(null);
    setTotpLoading(true);
    const result = await totpChallenge(totpEmail, totpCode);
    setTotpLoading(false);
    if (result?.error) {
      setTotpError(result.error);
    } else {
      router.push(postAuthDestination());
    }
  };

  //================================
  //
  //================================
  const handleOAuth = async (provider: "github" | "google") => {
    setServerError(null);
    const result = (await loginWithOAuth(provider)) as
      { error?: string; requiresTotp?: boolean } | undefined;

    if (result?.error) {
      setServerError(result.error);
      return;
    }
    if (result?.requiresTotp) {
      // Popup completed OAuth but the account needs its second step.
      setTotpEmail("");
      setTotpCode("");
      setTotpError(null);
      return;
    }
    router.push(postAuthDestination());
  };

  return (
    <>
      <AuthHeaderMobile />

      <div className="space-y-6">
        <div className="space-y-2">
          <h1 className="text-2xl font-bold tracking-tight">Welcome Back!</h1>
          <p className="text-muted-foreground">
            Sign in to access your dashboard and continue optimizing your
            notification workflow.
          </p>
        </div>

        {bannerMessage && <ErrorBanner message={bannerMessage} />}

        {totpEmail !== null ? (
          <form onSubmit={onTotpSubmit} className="space-y-4">
            <div className="space-y-2">
              <h2 className="text-lg font-semibold">Two-factor authentication</h2>
              <p className="text-sm text-muted-foreground">
                Enter the 6-digit code from your authenticator app
                {totpEmail ? (
                  <> for <span className="font-medium text-foreground">{totpEmail}</span></>
                ) : (
                  " for your account"
                )}
                .
              </p>
            </div>
            {totpEmail === "" && (
              <div className="space-y-2">
                <Label htmlFor="totp-email">Email</Label>
                <AuthInput
                  id="totp-email"
                  type="email"
                  icon={Mail}
                  placeholder="Enter your email"
                  value={totpEmail}
                  onChange={(e) => setTotpEmail(e.target.value)}
                />
              </div>
            )}
            <div className="space-y-2">
              <Label htmlFor="totp-code">Authentication code</Label>
              <AuthInput
                id="totp-code"
                icon={KeyRound}
                inputMode="numeric"
                autoComplete="one-time-code"
                placeholder="123456"
                value={totpCode}
                onChange={(e) => setTotpCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
              />
            </div>
            {totpError && <ErrorBanner message={totpError} />}
            <Button
              type="submit"
              className="h-10 w-full"
              disabled={totpLoading || totpCode.length !== 6 || !totpEmail}
            >
              {totpLoading ? (
                <>
                  <Loader2 className="mr-2 size-4 animate-spin" />
                  Verifying...
                </>
              ) : (
                "Verify & sign in"
              )}
            </Button>
            <Button
              type="button"
              variant="ghost"
              className="w-full"
              onClick={() => {
                setTotpEmail(null);
                setTotpCode("");
                setTotpError(null);
              }}
            >
              Back to sign in
            </Button>
          </form>
        ) : (
        <>
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
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

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label htmlFor="password">Password</Label>
              <Link
                href="/auth/password/forgot"
                className="text-xs text-muted-foreground hover:text-foreground"
              >
                Forgot password?
              </Link>
            </div>
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
          </div>

          <div className="flex items-center gap-2">
            <Checkbox
              id="remember"
              checked={rememberMe}
              onCheckedChange={(checked) =>
                setValue("rememberMe", checked === true)
              }
            />
            <Label htmlFor="remember" className="text-sm font-normal">
              Remember me
            </Label>
          </div>

          <Button type="submit" className="h-10 w-full" disabled={isLoading}>
            {isLoading ? (
              <>
                <Loader2 className="mr-2 size-4 animate-spin" />
                Signing in...
              </>
            ) : (
              "Sign in"
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
          onGitHub={() => handleOAuth("github")}
          onGoogle={() => handleOAuth("google")}
          isLoading={isLoading}
        />
        </> // end password-mode branch
        )}

        <AuthFooter mode="login" />
      </div>
    </>
  );
}
