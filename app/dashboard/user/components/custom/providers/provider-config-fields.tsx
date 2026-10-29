"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Check, Copy, RefreshCw } from "lucide-react";
import type { ProviderDefinition } from "@/hooks/use-provider-registry";
import { generateRouteSecret } from "@/lib/gateway";
import { GATEWAY_MODE_LABELS } from "@/lib/gateway";

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
 * Boolean toggles, dropdowns, text/password/email/number inputs with
 * required marks; non-empty non-secret values get a copy affordance.
 * The gateway card's route secret is a special case: generated prefilled
 * readonly, with a regenerate button beside the copy affordance.
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

  const gatewaySecret =
    definition.provider_id === "gateway" ? "secret_key" : null;

  return (
    <>
      {definition.config_fields.map((field) => {
        const value = values[field.key];
        // Hidden fields ride the recipe (the payload loops include them)
        // but never render — the dashboard generates them at connect.
        if (field.type === "hidden") {
          return null;
        }
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

        if (field.type === "select") {
          const options = field.options ?? [];
          const current =
            typeof value === "string" && options.includes(value)
              ? value
              : options[0];
          return (
            <div key={field.key} className="space-y-1.5">
              <Label className="text-sm font-medium">
                {field.label}
                {field.required && (
                  <span className="ml-1 text-destructive">*</span>
                )}
              </Label>
              <Select
                value={current}
                onValueChange={(v) => v && onChange(field.key, v)}
              >
                <SelectTrigger className="h-9 w-full">
                  <SelectValue placeholder={`Choose ${field.label.toLowerCase()}`} />
                </SelectTrigger>
                <SelectContent>
                  {options.map((option) => (
                    <SelectItem key={option} value={option}>
                      {GATEWAY_MODE_LABELS[option] ?? option}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          );
        }

        const text = toText(value);
        const isGatewaySecret = gatewaySecret === field.key;
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
                placeholder={
                  isGatewaySecret ? "Generated secret" : `Enter ${field.label.toLowerCase()}`
                }
                readOnly={isGatewaySecret}
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
              {isGatewaySecret && (
                <Button
                  variant="ghost"
                  size="icon-xs"
                  className="shrink-0"
                  onClick={() => onChange(field.key, generateRouteSecret())}
                  aria-label="Regenerate route secret"
                >
                  <RefreshCw className="size-3" />
                </Button>
              )}
            </div>
          </div>
        );
      })}
    </>
  );
}
