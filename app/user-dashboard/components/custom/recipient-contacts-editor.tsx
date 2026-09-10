"use client";

import { useMemo } from "react";
import { Plus, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

/**
 * Edited contact blob: a flat object of key -> string. Values are trimmed and
 * any non-string payload is stringified before editing.
 */
export type ContactsEditorValue = Record<string, string>;

/** Named fields rendered with friendly labels; everything else is "custom". */
const STRUCTURED_FIELDS: { key: string; label: string; placeholder: string }[] = [
  { key: "email", label: "Email", placeholder: "name@example.com" },
  { key: "phone", label: "Phone", placeholder: "+14155550100" },
  { key: "whatsapp", label: "WhatsApp", placeholder: "+14155550100" },
  { key: "androidToken", label: "Android push token", placeholder: "APA91b…" },
  { key: "iosToken", label: "iOS push token", placeholder: "e2f9…" },
];

const STRUCTURED_KEYS = new Set(STRUCTURED_FIELDS.map((f) => f.key));

/**
 * Converts arbitrary stored contacts into the editable string-keyed shape.
 */
export function contactsForEdit(
  contacts: Record<string, unknown> | undefined,
): ContactsEditorValue {
  const out: ContactsEditorValue = {};
  if (!contacts) return out;
  for (const [key, value] of Object.entries(contacts)) {
    if (typeof value === "string") {
      out[key] = value;
    } else if (value !== null && value !== undefined) {
      out[key] = JSON.stringify(value);
    }
  }
  return out;
}

/**
 * Structured fields (email, phone, what's-app, push tokens) plus an
 * "add custom key/value" section, all editing one flat contacts object.
 */
export function RecipientContactsEditor({
  value,
  onChange,
}: {
  value: ContactsEditorValue;
  onChange: (next: ContactsEditorValue) => void;
}) {
  const customKeys = useMemo(
    () =>
      Object.keys(value)
        .filter((k) => !STRUCTURED_KEYS.has(k))
        .sort(),
    [value],
  );

  const set = (key: string, val: string) => {
    const next: ContactsEditorValue = {};
    for (const [k, v] of Object.entries(value)) {
      if (k !== key) next[k] = v;
    }
    const trimmed = val.trim();
    if (trimmed) next[key] = trimmed;
    onChange(next);
  };

  const addCustom = () => {
    const next = { ...value };
    let i = 1;
    let key = `custom_${i}`;
    while (next[key] !== undefined) {
      i += 1;
      key = `custom_${i}`;
    }
    next[key] = "";
    onChange(next);
  };

  const removeCustom = (key: string) => {
    const next = { ...value };
    delete next[key];
    onChange(next);
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        {STRUCTURED_FIELDS.map((field) => (
          <div key={field.key} className="space-y-1.5">
            <Label htmlFor={`contact-${field.key}`}>{field.label}</Label>
            <Input
              id={`contact-${field.key}`}
              name={field.key}
              placeholder={field.placeholder}
              value={value[field.key] ?? ""}
              onChange={(e) => set(field.key, e.target.value)}
            />
          </div>
        ))}
      </div>

      {customKeys.length > 0 && (
        <div className="space-y-2">
          <Label>Custom fields</Label>
          {customKeys.map((key, idx) => (
            <div key={idx} className="flex items-center gap-2">
              <Input
                className="w-1/3 font-mono text-xs"
                aria-label={`Custom field ${idx + 1} name`}
                value={key}
                onChange={(e) => {
                  const next = { ...value };
                  const oldVal = next[key];
                  delete next[key];
                  const trimmed = e.target.value.trim();
                  if (trimmed && oldVal !== undefined) next[trimmed] = oldVal;
                  onChange(next);
                }}
              />
              <Input
                className="flex-1"
                aria-label={`Custom field ${idx + 1} value`}
                value={value[key]}
                placeholder="Value"
                onChange={(e) => set(key, e.target.value)}
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label={`Remove ${key}`}
                onClick={() => removeCustom(key)}
              >
                <X className="size-4" />
              </Button>
            </div>
          ))}
        </div>
      )}

      <Button type="button" variant="outline" size="sm" onClick={addCustom}>
        <Plus className="size-3.5 mr-1" /> Add custom field
      </Button>
    </div>
  );
}
