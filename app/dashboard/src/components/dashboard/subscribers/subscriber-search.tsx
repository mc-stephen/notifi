"use client";

import { Search } from "lucide-react";
import { Input } from "@/components/ui/input";

interface SubscriberSearchProps {
  value: string;
  onChange: (value: string) => void;
}

export function SubscriberSearch({ value, onChange }: SubscriberSearchProps) {
  return (
    <div className="relative">
      <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
      <Input
        placeholder="Search by ID, email, or phone…"
        className="pl-9"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </div>
  );
}
