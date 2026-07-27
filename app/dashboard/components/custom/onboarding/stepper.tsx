"use client";

import { Check } from "lucide-react";
import { cn } from "@/lib/utils";

type StepperProps = {
  currentStep: number;
  totalSteps: number;
  labels?: string[];
};

export function Stepper({ currentStep, totalSteps, labels }: StepperProps) {
  return (
    <div className="w-full">
      <div className="flex items-center justify-between">
        {Array.from({ length: totalSteps }).map((_, i) => (
          <div key={i} className="flex items-center">
            <div className="flex flex-col items-center">
              <div
                className={cn(
                  "flex size-8 items-center justify-center rounded-full border-2 text-xs font-medium transition-colors",
                  i < currentStep &&
                    "border-primary bg-primary text-primary-foreground",
                  i === currentStep &&
                    "border-primary bg-primary text-primary-foreground",
                  i > currentStep && "border-muted-foreground/30 text-muted-foreground"
                )}
              >
                {i < currentStep ? (
                  <Check className="size-4" />
                ) : (
                  <span>{i + 1}</span>
                )}
              </div>
              {labels && labels[i] && (
                <span
                  className={cn(
                    "mt-2 text-xs font-medium hidden sm:block",
                    i <= currentStep ? "text-foreground" : "text-muted-foreground"
                  )}
                >
                  {labels[i]}
                </span>
              )}
            </div>
            {i < totalSteps - 1 && (
              <div
                className={cn(
                  "mx-2 h-0.5 w-8 sm:w-12 md:w-16",
                  i < currentStep ? "bg-primary" : "bg-muted-foreground/30"
                )}
              />
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
