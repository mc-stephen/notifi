"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Check, Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";

type CodeBlockProps = {
  code: string;
  language?: string;
  className?: string;
  showCopy?: boolean;
};

export function CodeBlock({ code, language, className, showCopy = true }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className={cn("relative group rounded-lg border bg-muted/50", className)}>
      {language && (
        <div className="flex items-center justify-between border-b px-4 py-1.5">
          <span className="text-xs font-mono text-muted-foreground">{language}</span>
        </div>
      )}
      <div className="relative">
        <pre className="overflow-x-auto p-4 text-sm font-mono leading-relaxed">
          <code>{code}</code>
        </pre>
        {showCopy && (
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon-xs"
                  className="absolute right-2 top-2 opacity-0 transition-opacity group-hover:opacity-100"
                  onClick={handleCopy}
                />
              }
            >
              {copied ? <Check className="size-3" /> : <Copy className="size-3" />}
            </TooltipTrigger>
            <TooltipContent>{copied ? "Copied" : "Copy"}</TooltipContent>
          </Tooltip>
        )}
      </div>
    </div>
  );
}
