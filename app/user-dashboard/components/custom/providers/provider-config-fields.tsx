"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Check, Copy } from "lucide-react";
import type { ProviderDefinition } from "@/hooks/use-provider-registry";

export function toText(value: unknown): string {
  if (value === null || value === undefined) return "";
  if (typeof value === "boolean") return value ? "true" : "false";
  return String(value);
}

type ProviderConfigFieldsProps = {
  definition: ProviderDefinition;
  values: Record<string, unknown>;
  onChange: (key: string, value: unknown) => void;
};

/**
 * Shared credential-field renderer for provider connect + settings panels.
 * Boolean toggles, text/password/email/number inputs with required marks;
 * non-empty non-secret values get a copy affordance.
 */
export function ProviderConfigFields({
  definition,
  values,
  onChange,
}: ProviderConfigFieldsProps) {
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const handleCopy = async (key: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedKey(key);
      setTimeout(() => setCopiedKey(null), 2000);
    } catch {
      setCopiedKey(null);
    }
  };

  return (
    <>
      {definition.config_fields.map((field) => {
        const value = values[field.key];
        if (field.type === "boolean") {
          const checked = value === true || value === "true";
          return (
            <div key={field.key} className="flex items-center gap-2">
              <Checkbox
                checked={checked}
                onCheckedChange={(v) => onChange(field.key, v === true)}
              />
              <Label className="text-sm font-medium">
                {field.label}
                {field.required && (
                  <span className="ml-1 text-destructive">*</span>
                )}
              </Label>
            </div>
          );
        }

        const text = toText(value);
        return (
          <div key={field.key} className="space-y-1.5">
            <Label className="text-sm font-medium">
              {field.label}
              {field.required && (
                <span className="ml-1 text-destructive">*</span>
              )}
            </Label>
            <div className="flex items-center gap-1.5">
              <Input
                type={
                  field.type === "password"
                    ? "password"
                    : field.type === "email"
                      ? "email"
                      : field.type === "number"
                        ? "number"
                        : "text"
                }
                value={text}
                placeholder={`Enter ${field.label.toLowerCase()}`}
                onChange={(e) => onChange(field.key, e.target.value)}
                className="h-9 font-mono text-xs"
              />
              {field.type !== "password" && text && (
                <Button
                  variant="ghost"
                  size="icon-xs"
                  className="shrink-0"
                  onClick={() => handleCopy(field.key, text)}
                  aria-label={`Copy ${field.label}`}
                >
                  {copiedKey === field.key ? (
                    <Check className="size-3 text-success" />
                  ) : (
                    <Copy className="size-3" />
                  )}
                </Button>
              )}
            </div>
          </div>
        );
      })}
    </>
  );
}
