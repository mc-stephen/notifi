"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { PartyPopper, ArrowRight, BookOpen, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ErrorBanner } from "@/components/custom/auth/error-banner";
import { links } from "@/lib/env";
import { api } from "@/lib/api";
import { useOnboardingStore } from "@/store/onboarding-store";
import { useAuth } from "@/hooks/use-auth";

type ProjectSummary = { id: string; name: string };
type InviteReport = { sent: string[]; failed: { email: string; reason: string }[] };

export default function SuccessPage() {
  const router = useRouter();
  const {
    projectName,
    projectDescription,
    teamInvites,
    updateData,
    completeOnboarding: markComplete,
  } = useOnboardingStore();
  const { completeOnboarding } = useAuth();
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [inviteReport, setInviteReport] = useState<InviteReport | null>(null);

  const finish = () => {
    updateData({ teamInvites: [], teamEmails: [] });
    markComplete();
    router.push("/");
  };

  // Sends the invites collected on the invite-team step. The project only
  // exists after onboarding completes, so this runs here — and per-invite
  // failures (e.g. unknown account) never block entering the dashboard.
  const sendPendingInvites = async (project: string): Promise<InviteReport> => {
    const report: InviteReport = { sent: [], failed: [] };
    const { projects } = await api<{ projects: ProjectSummary[] }>("/app/projects");
    const wanted = project.trim() || "My App";
    const target =
      projects.find((p) => p.name === wanted) ?? projects[0];
    if (!target) return report;
    for (const invite of teamInvites) {
      try {
        await api(`/app/projects/${encodeURIComponent(target.id)}/members`, {
          method: "POST",
          body: JSON.stringify({ email: invite.email, role: invite.role }),
        });
        report.sent.push(invite.email);
      } catch (e) {
        report.failed.push({
          email: invite.email,
          reason: e instanceof Error ? e.message : "Could not invite this member.",
        });
      }
    }
    return report;
  };

  // Persisting the first project is what flips the server-side onboarding
  // flag that unlocks the dashboard.
  const handleGoToDashboard = async () => {
    setError(null);
    setSaving(true);

    // New projects always start in development mode (the project-level
    // environment gate); staging/production are toggled in the dashboard.
    const project = projectName.trim() || "My App";
    const result = await completeOnboarding({
      project: {
        name: project,
        description: projectDescription || null,
      },
    });

    if (result?.error) {
      setSaving(false);
      setError(result.error);
      return;
    }

    if (teamInvites.length > 0) {
      try {
        const report = await sendPendingInvites(project);
        setSaving(false);
        if (report.failed.length > 0) {
          setInviteReport(report);
          return;
        }
      } catch (e) {
        setSaving(false);
        setInviteReport({
          sent: [],
          failed: [
            {
              email: `${teamInvites.length} invite(s)`,
              reason:
                e instanceof Error ? e.message : "Could not reach the server.",
            },
          ],
        });
        return;
      }
    } else {
      setSaving(false);
    }

    finish();
  };

  return (
    <div className="flex flex-col items-center text-center">
      <div className="mb-6 flex size-16 items-center justify-center rounded-2xl bg-green-500/10 text-green-600 dark:text-green-400">
        <PartyPopper className="size-8" />
      </div>

      <h1 className="mb-2 text-3xl font-bold tracking-tight">
        You&apos;re all set!
      </h1>
      <p className="mb-8 max-w-md text-lg text-muted-foreground">
        Your Notifi workspace{" "}
        <span className="font-medium text-foreground">
          {projectName || "project"}
        </span>{" "}
        is ready. Start sending notifications right away.
      </p>

      {error && (
        <div className="mb-6 w-full max-w-sm text-left">
          <ErrorBanner message={error} />
        </div>
      )}

      {inviteReport && (
        <div className="mb-6 w-full max-w-sm rounded-xl border p-4 text-left">
          <p className="text-sm font-medium">
            {inviteReport.sent.length > 0
              ? `Sent ${inviteReport.sent.length} invite(s).`
              : "Some invites could not be sent."}
          </p>
          <ul className="mt-2 space-y-1">
            {inviteReport.failed.map((f) => (
              <li key={f.email} className="text-xs text-muted-foreground">
                <span className="font-medium text-foreground">{f.email}</span>
                {" — "}
                {f.reason}
              </li>
            ))}
          </ul>
          <p className="mt-2 text-xs text-muted-foreground">
            Tip: invites need an existing Notifi account. You can resend them
            anytime from the Team page.
          </p>
          <Button onClick={finish} className="mt-3 w-full">
            Continue to Dashboard
            <ArrowRight className="ml-2 size-4" />
          </Button>
        </div>
      )}

      <div className="flex w-full max-w-sm flex-col gap-3">
        <Button
          onClick={handleGoToDashboard}
          disabled={saving || inviteReport !== null}
          className="w-full"
        >
          {saving ? (
            <>
              <Loader2 className="mr-2 size-4 animate-spin" />
              Finishing up...
            </>
          ) : (
            <>
              Go to Dashboard
              <ArrowRight className="ml-2 size-4" />
            </>
          )}
        </Button>
        <Button
          variant="outline"
          onClick={() => window.open(links.docs, "_blank")}
          className="w-full"
        >
          <BookOpen className="mr-2 size-4" />
          Read the Docs
        </Button>
      </div>
    </div>
  );
}
