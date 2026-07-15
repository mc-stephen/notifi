import { LoginForm } from "@/components/auth/login-form";

export default function LoginPage() {
  return (
    <div className="relative flex min-h-screen items-center justify-center bg-background p-4">
      {/* Environment accent */}
      <div className="absolute left-0 right-0 top-0 h-0.5 bg-primary/60" />

      <div className="w-full max-w-sm space-y-6">
        <div className="rounded-2xl border border-glass-border bg-glass backdrop-blur-[var(--blur-glass)] p-8">
          <div className="space-y-2 text-center mb-6">
            <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-primary text-lg font-bold text-primary-foreground shadow-sm">
              N
            </div>
            <h1 className="text-xl font-semibold tracking-tight">Welcome back</h1>
            <p className="text-sm text-muted-foreground">
              Sign in to your Notifi dashboard
            </p>
          </div>
          <LoginForm />
        </div>
        <p className="text-center text-xs text-muted-foreground">
          Notifi Notification Service &mdash; NaaS
        </p>
      </div>
    </div>
  );
}
