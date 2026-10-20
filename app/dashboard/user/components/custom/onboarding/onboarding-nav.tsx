"use client";

import { useRouter } from "next/navigation";
import { ArrowLeft, ArrowRight, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useOnboardingStore } from "@/store/onboarding-store";

type OnboardingNavProps = {
  showBack?: boolean;
  showNext?: boolean;
  nextLabel?: string;
  isLoading?: boolean;
  /** Disables the Continue button until the step's input is valid. */
  nextDisabled?: boolean;
  onNext?: () => void;
  onBack?: () => void;
};

export function OnboardingNav({
  showBack = true,
  showNext = true,
  nextLabel = "Continue",
  isLoading = false,
  nextDisabled = false,
  onNext,
  onBack,
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
        {showNext && (
          <Button
            onClick={handleNext}
            disabled={isLoading || nextDisabled}
          >
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
