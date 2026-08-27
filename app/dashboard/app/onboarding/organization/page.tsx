"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Building2, Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { OnboardingNav } from "@/components/custom/onboarding/onboarding-nav";
import { useOnboardingStore } from "@/store/onboarding-store";

export default function OrganizationPage() {
  const router = useRouter();
  const { orgName, orgLogo, updateData } = useOnboardingStore();
  const [name, setName] = useState(orgName);

  const handleContinue = () => {
    updateData({ orgName: name });
    useOnboardingStore.getState().nextStep();
    router.push(
      useOnboardingStore
        .getState()
        .getStepRoute(useOnboardingStore.getState().currentStep)
    );
  };

  return (
    <div className="flex flex-col">
      <h1 className="mb-2 text-2xl font-bold tracking-tight">
        Organization details
      </h1>
      <p className="mb-8 text-muted-foreground">
        Tell us about your company or team.
      </p>

      <div className="mb-10 space-y-6">
        {/* Logo upload */}
        <div className="flex items-center gap-4">
          <div className="flex size-16 items-center justify-center rounded-xl border-2 border-dashed bg-muted/50">
            {orgLogo ? (
              <img
                src={orgLogo}
                alt="Organization logo"
                className="size-full rounded-xl object-cover"
              />
            ) : (
              <Building2 className="size-6 text-muted-foreground" />
            )}
          </div>
          <div>
            <Button variant="outline" size="sm">
              <Upload className="mr-2 size-3" />
              Upload logo
            </Button>
            <p className="mt-1 text-xs text-muted-foreground">
              Square, 256×256px or larger
            </p>
          </div>
        </div>

        {/* Organization name */}
        <div className="space-y-2">
          <Label htmlFor="org-name">Organization name</Label>
          <Input
            id="org-name"
            placeholder="Acme Corp"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
      </div>

      <OnboardingNav
        onNext={handleContinue}
        nextLabel="Continue"
        nextDisabled={!name.trim()}
      />
    </div>
  );
}
