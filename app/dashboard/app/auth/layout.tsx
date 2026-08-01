import type { Metadata } from "next";
import { AuthHeader } from "@/components/custom/auth/auth-header";

export const metadata: Metadata = {
  title: "Authentication",
  description: "Sign in to your Notifi account",
};

export default function AuthLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="flex min-h-screen">
      <div className="relative flex flex-1 flex-col items-center justify-center overflow-hidden p-6 lg:w-1/2">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 [background-image:radial-gradient(circle,var(--border)_1px,transparent_1px)] [background-size:24px_24px] [mask-image:radial-gradient(ellipse_at_center,transparent_10%,black_70%)] opacity-60"
        />
        <div className="relative w-full max-w-sm">{children}</div>
      </div>
      <AuthHeader />
    </div>
  );
}
