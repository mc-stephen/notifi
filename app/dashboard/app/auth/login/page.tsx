"use client";

import * as z from "zod";
import Link from "next/link";
import { useState } from "react";
import { useRouter } from "next/navigation";
import { Loader2, Mail } from "lucide-react";
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
  const router = useRouter();
  const { login, loginWithOAuth, isLoading } = useAuth();
  const [serverError, setServerError] = useState<string | null>(null);

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
      { error?: string } | undefined;

    if (result?.error) {
      setServerError(result.error);
    } else {
      router.push("/");
    }
  };

  //================================
  //
  //================================
  const handleOAuth = async (provider: "github" | "google") => {
    setServerError(null);
    const result = (await loginWithOAuth(provider)) as
      { error?: string } | undefined;

    if (result?.error) {
      setServerError(result.error);
      return;
    }
    router.push("/");
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

        {serverError && <ErrorBanner message={serverError} />}

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

        <AuthFooter mode="login" />
      </div>
    </>
  );
}
