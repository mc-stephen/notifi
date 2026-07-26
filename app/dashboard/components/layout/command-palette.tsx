"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Command } from "cmdk";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { useUIStore } from "@/store/ui-store";
import { NAV_GROUPS } from "@/lib/constants";
import { Search } from "lucide-react";

export function CommandPalette() {
  const { commandOpen, setCommandOpen } = useUIStore();
  const router = useRouter();
  const [search, setSearch] = useState("");

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setCommandOpen(!commandOpen);
      }
      if (e.key === "Escape") {
        setCommandOpen(false);
      }
    };

    document.addEventListener("keydown", down);
    return () => document.removeEventListener("keydown", down);
  }, [commandOpen, setCommandOpen]);

  const handleSelect = (href: string) => {
    router.push(href);
    setCommandOpen(false);
    setSearch("");
  };

  return (
    <Dialog open={commandOpen} onOpenChange={setCommandOpen}>
      <DialogContent className="gap-0 p-0 overflow-hidden max-w-lg">
        <Command
          value={search}
          onValueChange={setSearch}
          filter={(value, search) => {
            if (value.toLowerCase().includes(search.toLowerCase())) return 1;
            return 0;
          }}
        >
          <div className="flex items-center border-b px-3">
            <Search className="size-4 shrink-0 text-muted-foreground" />
            <Command.Input
              placeholder="Search pages..."
              className="flex h-11 w-full rounded-md bg-transparent py-3 pl-2 text-sm outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50"
            />
          </div>
          <Command.List className="max-h-[300px] overflow-y-auto p-2">
            <Command.Empty className="py-6 text-center text-sm text-muted-foreground">
              No results found.
            </Command.Empty>
            {NAV_GROUPS.map((group) => (
              <Command.Group key={group.label} heading={group.label} className="px-1">
                {group.items.map((item) => (
                  <Command.Item
                    key={item.href}
                    value={item.label}
                    onSelect={() => handleSelect(item.href)}
                    className="relative flex cursor-pointer select-none items-center gap-2 rounded-md px-2 py-1.5 text-sm outline-none aria-selected:bg-accent aria-selected:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
                  >
                    <item.icon className="size-4 text-muted-foreground" />
                    <span>{item.label}</span>
                  </Command.Item>
                ))}
              </Command.Group>
            ))}
          </Command.List>
          <div className="border-t px-3 py-2">
            <p className="text-xs text-muted-foreground">
              Press <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-0.5 rounded border bg-muted px-1.5 text-[10px] font-medium">ESC</kbd> to close
            </p>
          </div>
        </Command>
      </DialogContent>
    </Dialog>
  );
}
