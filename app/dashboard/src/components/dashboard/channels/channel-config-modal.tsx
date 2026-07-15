"use client";

import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { ChannelConfig } from "@/lib/types";

interface ChannelConfigModalProps {
  channel: ChannelConfig | null;
  open: boolean;
  onSave: (id: string) => void;
  onCancel: () => void;
}

export function ChannelConfigModal({
  channel,
  open,
  onSave,
  onCancel,
}: ChannelConfigModalProps) {
  if (!channel) return null;

  const schema = z.object(
    Object.fromEntries(
      channel.configFields.map((f) => [
        f.key,
        f.required
          ? z.string().min(1, `${f.label} is required`)
          : z.string().optional(),
      ]),
    ),
  );

  type FormData = z.infer<typeof schema>;

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<FormData>({
    resolver: zodResolver(schema),
  });

  const onSubmit = async () => {
    await new Promise((r) => setTimeout(r, 600));
    onSave(channel.id);
  };

  return (
    <Dialog open={open} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Configure {channel.provider}</DialogTitle>
          <DialogDescription>
            Enter the credentials for your {channel.channel} provider.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          {channel.configFields.map((field) => (
            <div key={field.key} className="space-y-2">
              <Label htmlFor={field.key}>
                {field.label}
                {field.required && (
                  <span className="text-destructive ml-0.5">*</span>
                )}
              </Label>
              {field.type === "file" ? (
                <div className="flex h-10 w-full items-center justify-center rounded-md border border-dashed border-border bg-secondary/50 px-3 text-xs text-muted-foreground cursor-pointer hover:bg-secondary transition-colors">
                  Click to upload {field.label}
                </div>
              ) : (
                <Input
                  id={field.key}
                  type={field.type}
                  placeholder={field.placeholder}
                  {...register(field.key)}
                />
              )}
              {errors[field.key] && (
                <p className="text-xs text-destructive">
                  {errors[field.key]?.message as string}
                </p>
              )}
            </div>
          ))}

          <div className="flex justify-end gap-2 pt-2">
            <Button type="button" variant="outline" onClick={onCancel}>
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? "Saving…" : "Save Configuration"}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
