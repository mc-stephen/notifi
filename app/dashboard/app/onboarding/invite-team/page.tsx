"use client";

import { useState } from "react";
import { X, UserPlus, Users } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { OnboardingNav } from "@/components/custom/onboarding/onboarding-nav";
import { useOnboardingStore } from "@/store/onboarding-store";

const ROLES = [
  { value: "admin", label: "Admin" },
  { value: "editor", label: "Editor" },
  { value: "viewer", label: "Viewer" },
];

type Invite = { email: string; role: string };

export default function InviteTeamPage() {
  const { teamEmails, updateData } = useOnboardingStore();
  const [invites, setInvites] = useState<Invite[]>(
    teamEmails.length > 0
      ? teamEmails.map((email) => ({ email, role: "viewer" }))
      : []
  );
  const [emailInput, setEmailInput] = useState("");
  const [roleInput, setRoleInput] = useState("viewer");

  const addInvite = () => {
    const trimmed = emailInput.trim();
    if (!trimmed || invites.some((i) => i.email === trimmed)) return;
    setInvites([...invites, { email: trimmed, role: roleInput }]);
    setEmailInput("");
  };

  const removeInvite = (email: string) => {
    setInvites(invites.filter((i) => i.email !== email));
  };

  const handleContinue = () => {
    updateData({ teamEmails: invites.map((i) => i.email) });
    useOnboardingStore.getState().nextStep();
    window.location.href = useOnboardingStore
      .getState()
      .getStepRoute(useOnboardingStore.getState().currentStep);
  };

  return (
    <div className="flex flex-col">
      <div className="mb-6 flex items-center gap-3">
        <div className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <Users className="size-5" />
        </div>
        <div>
          <h1 className="text-2xl font-bold tracking-tight">
            Invite your team
          </h1>
          <p className="text-sm text-muted-foreground">
            Collaborate with your team members. You can also do this later.
          </p>
        </div>
      </div>

      <div className="mb-10 space-y-6">
        {/* Add invite form */}
        <div className="flex gap-2">
          <div className="flex-1">
            <Input
              placeholder="colleague@company.com"
              type="email"
              value={emailInput}
              onChange={(e) => setEmailInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  addInvite();
                }
              }}
            />
          </div>
          <Select value={roleInput} onValueChange={(v) => v && setRoleInput(v)}>
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ROLES.map((r) => (
                <SelectItem key={r.value} value={r.value}>
                  {r.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button variant="outline" onClick={addInvite}>
            <UserPlus className="size-4" />
          </Button>
        </div>

        {/* Invite list */}
        {invites.length > 0 && (
          <div className="space-y-2">
            <Label>Invited ({invites.length})</Label>
            {invites.map((inv) => (
              <div
                key={inv.email}
                className="flex items-center justify-between rounded-lg border px-3 py-2"
              >
                <div className="flex items-center gap-3">
                  <div className="flex size-7 items-center justify-center rounded-full bg-muted text-xs font-medium">
                    {inv.email.charAt(0).toUpperCase()}
                  </div>
                  <div>
                    <div className="text-sm font-medium">{inv.email}</div>
                    <div className="text-xs capitalize text-muted-foreground">
                      {inv.role}
                    </div>
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="size-7 p-0"
                  onClick={() => removeInvite(inv.email)}
                >
                  <X className="size-3" />
                </Button>
              </div>
            ))}
          </div>
        )}

        {invites.length === 0 && (
          <div className="rounded-xl border border-dashed p-8 text-center">
            <Users className="mx-auto mb-2 size-8 text-muted-foreground/40" />
            <p className="text-sm text-muted-foreground">
              No team members invited yet. Add emails above to invite them.
            </p>
          </div>
        )}
      </div>

      <OnboardingNav
        showSkip
        onNext={handleContinue}
        nextLabel="Continue"
      />
    </div>
  );
}
