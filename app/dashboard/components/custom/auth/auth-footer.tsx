import Link from "next/link";

type AuthFooterProps = {
  mode: "login" | "signup";
};

export function AuthFooter({ mode }: AuthFooterProps) {
  return (
    <div className="mt-6 text-center text-sm text-muted-foreground">
      {mode === "login" ? (
        <p>
          Don&apos;t have an account?{" "}
          <Link
            href="/auth/signup"
            className="font-medium text-primary hover:underline"
          >
            Sign up
          </Link>
        </p>
      ) : (
        <p>
          Already have an account?{" "}
          <Link
            href="/auth/login"
            className="font-medium text-primary hover:underline"
          >
            Sign in
          </Link>
        </p>
      )}
    </div>
  );
}
