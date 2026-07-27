"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Code2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { OnboardingNav } from "@/components/custom/onboarding/onboarding-nav";
import { useOnboardingStore } from "@/store/onboarding-store";

export default function ProjectPage() {
  const router = useRouter();
  const {
    projectName,
    projectDescription,
    projectEnvironment,
    updateData,
  } = useOnboardingStore();

  const [name, setName] = useState(projectName);
  const [description, setDescription] = useState(projectDescription);
  const [environment, setEnvironment] = useState(projectEnvironment);

  const handleContinue = () => {
    updateData({
      projectName: name,
      projectDescription: description,
      projectEnvironment: environment,
    });
    useOnboardingStore.getState().nextStep();
    router.push(
      useOnboardingStore
        .getState()
        .getStepRoute(useOnboardingStore.getState().currentStep)
    );
  };

  return (
    <div className="flex flex-col">
      <div className="mb-6 flex items-center gap-3">
        <div className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <Code2 className="size-5" />
        </div>
        <div>
          <h1 className="text-2xl font-bold tracking-tight">
            Create your first project
          </h1>
          <p className="text-sm text-muted-foreground">
            Projects organize your notifications, channels, and team access.
          </p>
        </div>
      </div>

      <div className="mb-10 space-y-6">
        {/* Project name */}
        <div className="space-y-2">
          <Label htmlFor="project-name">Project name</Label>
          <Input
            id="project-name"
            placeholder="My App"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            A descriptive name to identify this project.
          </p>
        </div>

        {/* Description */}
        <div className="space-y-2">
          <Label htmlFor="project-desc">Description</Label>
          <Textarea
            id="project-desc"
            placeholder="What does this project do?"
            rows={3}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </div>

        {/* Environment */}
        <div className="space-y-2">
          <Label>Default environment</Label>
          <Select value={environment} onValueChange={(v) => v && setEnvironment(v)}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="development">Development</SelectItem>
              <SelectItem value="staging">Staging</SelectItem>
              <SelectItem value="production">Production</SelectItem>
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            You can create additional environments later.
          </p>
        </div>

        {/* Quick preview */}
        <div className="rounded-xl border bg-muted/30 p-4">
          <div className="mb-2 text-xs font-medium text-muted-foreground">
            Project Preview
          </div>
          <div className="flex items-center gap-3">
            <div className="flex size-10 items-center justify-center rounded-lg bg-primary text-primary-foreground text-sm font-bold">
              {name ? name.charAt(0).toUpperCase() : "P"}
            </div>
            <div>
              <div className="font-medium">{name || "My App"}</div>
              <div className="text-xs text-muted-foreground">
                {description || "No description"} · {environment}
              </div>
            </div>
          </div>
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
