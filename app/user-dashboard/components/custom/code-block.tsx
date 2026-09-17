"use client";

import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import hljs from "highlight.js/lib/core";
import typescript from "highlight.js/lib/languages/typescript";
import python from "highlight.js/lib/languages/python";
import go from "highlight.js/lib/languages/go";
import bash from "highlight.js/lib/languages/bash";
import json from "highlight.js/lib/languages/json";
import { cn } from "@/lib/utils";
import { Check, Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";

hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("python", python);
hljs.registerLanguage("go", go);
hljs.registerLanguage("bash", bash);
hljs.registerLanguage("json", json);

/// next-themes toggles `.dark` on <html> (attribute="class"). Subscribing to
/// that class keeps code blocks in sync with the dashboard theme without a
/// mounted-guard setState (React Compiler clean) and without re-highlighting.
function subscribeToTheme(callback: () => void) {
  const observer = new MutationObserver(callback);
  observer.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ["class"],
  });
  return () => observer.disconnect();
}

function getThemeSnapshot() {
  return document.documentElement.classList.contains("dark");
}

function getServerThemeSnapshot() {
  return false;
}

type CodeBlockProps = {
  code: string;
  language?: string;
  className?: string;
  showCopy?: boolean;
};

export function CodeBlock({ code, language, className, showCopy = true }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  const dark = useSyncExternalStore(
    subscribeToTheme,
    getThemeSnapshot,
    getServerThemeSnapshot,
  );
  const codeRef = useRef<HTMLElement>(null);

  const requested = (language ?? "plaintext").toLowerCase();
  const resolved = hljs.getLanguage(requested) ? requested : "plaintext";

  useEffect(() => {
    const el = codeRef.current;
    if (!el) return;
    el.removeAttribute("data-highlighted");
    hljs.highlightElement(el);
  }, [code, resolved]);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div
      className={cn(
        "relative group overflow-hidden rounded-lg border",
        dark
          ? "codeblock-dark border-border bg-[#0d1117]"
          : "codeblock-light border-border bg-popover",
        className,
      )}
    >
      {language && (
        <div
          className={cn(
            "flex items-center justify-between px-4 py-1.5",
            dark ? "border-b border-white/10" : "border-b",
          )}
        >
          <span className={cn("text-xs font-mono", dark ? "text-gray-400" : "text-muted-foreground")}>
            {requested}
          </span>
        </div>
      )}
      <div className="relative">
        <pre className="overflow-x-auto p-4 font-mono text-[13px] leading-relaxed">
          <code ref={codeRef} className={`language-${resolved}`}>
            {code}
          </code>
        </pre>
        {showCopy && (
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon-xs"
                  className={cn(
                    "absolute right-2 top-2 opacity-0 transition-opacity group-hover:opacity-100",
                    dark
                      ? "text-gray-400 hover:bg-white/10 hover:text-white"
                      : "text-muted-foreground hover:text-foreground",
                  )}
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
