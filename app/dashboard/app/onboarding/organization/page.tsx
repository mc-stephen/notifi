"use client";

import { useState } from "react";
import { Building2, Upload } from "lucide-react";
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

const REGIONS = [
  { value: "us-east-1", label: "US East (Virginia)" },
  { value: "us-west-2", label: "US West (Oregon)" },
  { value: "eu-west-1", label: "EU (Ireland)" },
  { value: "eu-central-1", label: "EU (Frankfurt)" },
  { value: "ap-southeast-1", label: "Asia Pacific (Singapore)" },
  { value: "ap-northeast-1", label: "Asia Pacific (Tokyo)" },
];

const TIMEZONES = [
  "America/New_York",
  "America/Chicago",
  "America/Denver",
  "America/Los_Angeles",
  "Europe/London",
  "Europe/Berlin",
  "Asia/Tokyo",
  "Asia/Singapore",
  "Australia/Sydney",
];

export default function OrganizationPage() {
  const { orgName, orgLogo, region, timezone, updateData } = useOnboardingStore();
  const [name, setName] = useState(orgName);
  const [selectedRegion, setSelectedRegion] = useState(region);
  const [selectedTimezone, setSelectedTimezone] = useState(
    timezone || "America/New_York"
  );

  const handleContinue = () => {
    updateData({
      orgName: name,
      region: selectedRegion,
      timezone: selectedTimezone,
    });
    useOnboardingStore.getState().nextStep();
    window.location.href = useOnboardingStore
      .getState()
      .getStepRoute(useOnboardingStore.getState().currentStep);
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

        {/* Region */}
        <div className="space-y-2">
          <Label>Region</Label>
          <Select value={selectedRegion} onValueChange={(v) => v && setSelectedRegion(v)}>
            <SelectTrigger>
              <SelectValue placeholder="Select a region" />
            </SelectTrigger>
            <SelectContent>
              {REGIONS.map((r) => (
                <SelectItem key={r.value} value={r.value}>
                  {r.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            Data residency region. Notifications will be sent from this region.
          </p>
        </div>

        {/* Timezone */}
        <div className="space-y-2">
          <Label>Timezone</Label>
          <Select value={selectedTimezone} onValueChange={(v) => v && setSelectedTimezone(v)}>
            <SelectTrigger>
              <SelectValue placeholder="Select a timezone" />
            </SelectTrigger>
            <SelectContent>
              {TIMEZONES.map((tz) => (
                <SelectItem key={tz} value={tz}>
                  {tz.replace(/_/g, " ")}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      <OnboardingNav
        showSkip
        onNext={handleContinue}
        nextLabel="Continue"
      />
    </div>
  );
}
