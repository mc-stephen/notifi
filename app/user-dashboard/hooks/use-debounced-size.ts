"use client";

import { useEffect, useRef, useState } from "react";

/**
 * Measures an element with a trailing-debounce ResizeObserver.
 *
 * Used by chart components: passing explicit pixel dimensions to Recharts'
 * `ResponsiveContainer` disables its internal per-tick observer, so charts
 * re-render at most twice per resize burst (e.g. the 150ms sidebar
 * collapse) instead of every animation frame — then settle pixel-correct.
 * The initial measurement applies immediately (no debounce on mount).
 */
export function useDebouncedSize<T extends HTMLElement>(delay = 120) {
  const ref = useRef<T | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const measure = () => {
      const rect = el.getBoundingClientRect();
      const width = Math.max(0, Math.floor(rect.width));
      const height = Math.max(0, Math.floor(rect.height));
      setSize((prev) =>
        prev.width === width && prev.height === height
          ? prev
          : { width, height },
      );
    };

    measure();

    let timer: ReturnType<typeof setTimeout> | null = null;
    const observer = new ResizeObserver(() => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        timer = null;
        measure();
      }, delay);
    });
    observer.observe(el);

    return () => {
      if (timer) clearTimeout(timer);
      observer.disconnect();
    };
  }, [delay]);

  return { ref, width: size.width, height: size.height };
}
