"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import { usePathname, useSearchParams } from "next/navigation";
import { motion, AnimatePresence } from "framer-motion";
import { cn } from "@/lib/utils";

export function NavigationProgress({ className }: { className?: string }) {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const [progress, setProgress] = useState(0);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const prevPathname = useRef(pathname);

  // Complete progress when pathname/searchParams change
  useEffect(() => {
    if (prevPathname.current !== pathname) {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      setProgress(100);
      prevPathname.current = pathname;
    }
  }, [pathname]);

  // Reset progress after completion
  useEffect(() => {
    if (progress === 100) {
      const timer = setTimeout(() => setProgress(0), 400);
      return () => clearTimeout(timer);
    }
  }, [progress]);

  // Start progress
  const startProgress = useCallback(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
    }

    setProgress(30);

    let current = 30;
    intervalRef.current = setInterval(() => {
      current += Math.random() * 20 + 5;
      if (current >= 85) {
        current = 85;
        if (intervalRef.current) {
          clearInterval(intervalRef.current);
          intervalRef.current = null;
        }
      }
      setProgress(current);
    }, 200);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, []);

  // Listen for link clicks
  useEffect(() => {
    const handleAnchorClick = (e: MouseEvent) => {
      const target = e.currentTarget as HTMLAnchorElement;
      if (!target) return;

      const href = target.getAttribute("href");
      if (!href) return;

      const isInternal = href.startsWith("/") && !href.startsWith("//");
      const isExternal = target.target === "_blank";
      const isSamePath =
        href === pathname ||
        href === `${pathname}#` ||
        href.startsWith("#");

      if (isInternal && !isExternal && !isSamePath) {
        startProgress();
      }
    };

    const attachListeners = () => {
      const anchors = document.querySelectorAll("a[href]");
      anchors.forEach((anchor) => {
        anchor.addEventListener(
          "click",
          handleAnchorClick as EventListener
        );
      });
    };

    attachListeners();
    const observer = new MutationObserver(attachListeners);
    observer.observe(document.body, {
      childList: true,
      subtree: true,
    });

    return () => {
      observer.disconnect();
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
      const anchors = document.querySelectorAll("a[href]");
      anchors.forEach((anchor) => {
        anchor.removeEventListener(
          "click",
          handleAnchorClick as EventListener
        );
      });
    };
  }, [pathname, startProgress]);

  return (
    <AnimatePresence>
      {progress > 0 && (
        <motion.div
          className={cn(
            "pointer-events-none h-[3px]",
            className
          )}
          initial={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.3 }}
        >
          <motion.div
            className="h-full bg-primary rounded-full"
            initial={{ width: "0%", opacity: 1 }}
            animate={{ width: `${progress}%`, opacity: 1 }}
            transition={{
              width: { duration: 0.4, ease: "easeOut" },
            }}
          />
          <motion.div
            className="absolute top-0 right-0 h-full w-16 bg-gradient-to-r from-transparent via-primary/50 to-primary blur-sm"
            animate={{ left: `${progress - 15}%` }}
            transition={{ duration: 0.4, ease: "easeOut" }}
          />
        </motion.div>
      )}
    </AnimatePresence>
  );
}
