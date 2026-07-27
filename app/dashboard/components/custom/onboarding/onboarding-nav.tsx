"use client";

import { useRouter } from "next/navigation";
import { ArrowLeft, ArrowRight, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useOnboardingStore } from "@/store/onboarding-store";

type OnboardingNavProps = {
  showBack?: boolean;
  showSkip?: boolean;
  showNext?: boolean;
  nextLabel?: string;
  isLoading?: boolean;
  onNext?: () => void;
  onBack?: () => void;
  onSkip?: () => void;
};

export function OnboardingNav({
  showBack = true,
  showSkip = false,
  showNext = true,
  nextLabel = "Continue",
  isLoading = false,
  onNext,
  onBack,
  onSkip,
}: OnboardingNavProps) {
  const router = useRouter();
  const { nextStep, prevStep, getStepRoute, currentStep, getTotalSteps } =
    useOnboardingStore();

  const isLastStep = currentStep === getTotalSteps() - 1;

  const handleNext = () => {
    if (onNext) {
      onNext();
    } else {
      nextStep();
      router.push(getStepRoute(currentStep + 1));
    }
  };

  const handleBack = () => {
    if (onBack) {
      onBack();
    } else {
      prevStep();
      router.push(getStepRoute(currentStep - 1));
    }
  };

  const handleSkip = () => {
    if (onSkip) {
      onSkip();
    } else {
      router.push("/");
    }
  };

  return (
    <div className="flex items-center justify-between">
      <div>
        {showBack && currentStep > 0 && (
          <Button variant="ghost" onClick={handleBack} disabled={isLoading}>
            <ArrowLeft className="mr-2 size-4" />
            Back
          </Button>
        )}
      </div>

      <div className="flex items-center gap-3">
        {showSkip && (
          <Button variant="ghost" onClick={handleSkip} disabled={isLoading}>
            Skip for now
          </Button>
        )}
        {showNext && (
          <Button onClick={handleNext} disabled={isLoading}>
            {isLoading ? (
              <>
                <Loader2 className="mr-2 size-4 animate-spin" />
                Processing...
              </>
            ) : (
              <>
                {nextLabel}
                {!isLastStep && <ArrowRight className="ml-2 size-4" />}
              </>
            )}
          </Button>
        )}
      </div>
    </div>
  );
}
