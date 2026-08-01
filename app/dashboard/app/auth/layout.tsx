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
      <div className="flex flex-1 flex-col items-center justify-center p-6 lg:w-1/2">
        <div className="w-full max-w-sm">{children}</div>
      </div>
      <AuthHeader />
    </div>
  );
}
