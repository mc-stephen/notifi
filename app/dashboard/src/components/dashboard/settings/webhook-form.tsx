"use client";

import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";

const webhookSchema = z.object({
  url: z.string().url("Enter a valid URL"),
  events: z.array(z.string()).min(1, "Select at least one event"),
});

type WebhookFormData = z.infer<typeof webhookSchema>;

interface WebhookFormProps {
  availableEvents: readonly string[];
  onSubmit: (url: string, events: string[]) => void;
  onCancel: () => void;
}

export function WebhookForm({ availableEvents, onSubmit, onCancel }: WebhookFormProps) {
  const {
    register,
    handleSubmit,
    watch,
    setValue,
    formState: { errors },
  } = useForm<WebhookFormData>({
    resolver: zodResolver(webhookSchema),
    defaultValues: { url: "", events: [] },
  });

  const selectedEvents = watch("events");

  const toggleEvent = (event: string) => {
    const current = selectedEvents || [];
    const next = current.includes(event)
      ? current.filter((e) => e !== event)
      : [...current, event];
    setValue("events", next, { shouldValidate: true });
  };

  const onFormSubmit = (data: WebhookFormData) => {
    onSubmit(data.url, data.events);
  };

  return (
    <form onSubmit={handleSubmit(onFormSubmit)} className="space-y-4">
      <div className="space-y-2">
        <Label htmlFor="url">Endpoint URL</Label>
        <Input
          id="url"
          type="url"
          placeholder="https://api.example.com/webhook"
          {...register("url")}
        />
        {errors.url && (
          <p className="text-xs text-destructive">{errors.url.message}</p>
        )}
      </div>

      <div className="space-y-2">
        <Label>Events</Label>
        <div className="grid grid-cols-2 gap-2">
          {availableEvents.map((event) => (
            <label
              key={event}
              className="flex items-center gap-2 rounded-md border border-border px-3 py-2 cursor-pointer hover:bg-secondary transition-colors text-sm"
            >
              <Checkbox
                checked={selectedEvents?.includes(event)}
                onCheckedChange={() => toggleEvent(event)}
              />
              {event}
            </label>
          ))}
        </div>
        {errors.events && (
          <p className="text-xs text-destructive">{errors.events.message}</p>
        )}
      </div>

      <div className="flex justify-end gap-2 pt-2">
        <Button type="button" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit">Add Webhook</Button>
      </div>
    </form>
  );
}
