"use client";

import { Check } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  validatePassword,
  getPasswordStrength,
  type PasswordRequirements,
} from "@/lib/auth-types";

type PasswordStrengthProps = {
  password: string;
};

const REQUIREMENTS = [
  { key: "minLength" as keyof PasswordRequirements, label: "At least 8 characters" },
  { key: "hasUppercase" as keyof PasswordRequirements, label: "At least 1 uppercase letter" },
  { key: "hasLowercase" as keyof PasswordRequirements, label: "At least 1 lowercase letter" },
  { key: "hasNumber" as keyof PasswordRequirements, label: "At least 1 number" },
  { key: "hasSpecialChar" as keyof PasswordRequirements, label: "At least 1 special character" },
];

export function PasswordStrength({ password }: PasswordStrengthProps) {
  if (!password) return null;

  const requirements = validatePassword(password);
  const { score, label, color } = getPasswordStrength(requirements);

  return (
    <div className="space-y-3">
      <div className="space-y-1.5">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">Password strength</span>
          <span
            className={cn(
              "font-medium",
              score <= 1 && "text-destructive",
              score === 2 && "text-orange-500",
              score === 3 && "text-yellow-500",
              score >= 4 && "text-success"
            )}
          >
            {label}
          </span>
        </div>
        <div className="flex gap-1">
          {Array.from({ length: 5 }).map((_, i) => (
            <div
              key={i}
              className={cn(
                "h-1 flex-1 rounded-full transition-colors",
                i < score ? color : "bg-muted"
              )}
            />
          ))}
        </div>
      </div>

      <div className="space-y-1.5">
        {REQUIREMENTS.map(({ key, label }) => {
          const met = requirements[key];
          return (
            <div key={key} className="flex items-center gap-2 text-xs">
              <div
                className={cn(
                  "flex size-3.5 items-center justify-center rounded-full border",
                  met
                    ? "border-success bg-success text-primary-foreground"
                    : "border-muted-foreground/30"
                )}
              >
                {met && <Check className="size-2.5" />}
              </div>
              <span className={cn(met ? "text-foreground" : "text-muted-foreground")}>
                {label}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
