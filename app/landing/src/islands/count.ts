export {}

/**
 * Animated counters for stat/metric values.
 * Progressive enhancement: markup always contains the final value;
 * the script animates from 0 only when JS runs and motion is allowed.
 */
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches
const targets = document.querySelectorAll<HTMLElement>("[data-count]")

const format = (el: HTMLElement, value: number) => {
  const decimals = parseInt(el.getAttribute("data-decimals") ?? "0", 10)
  const prefix = el.getAttribute("data-prefix") ?? ""
  const suffix = el.getAttribute("data-suffix") ?? ""
  return (
    prefix +
    value.toLocaleString("en-US", {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    }) +
    suffix
  )
}

if (targets.length && !reduceMotion && "IntersectionObserver" in window) {
  /* Start at 0 immediately so the SSR final value never flashes before
     the count-up begins (the elements are below the fold at load). */
  targets.forEach((el) => {
    el.textContent = format(el, 0)
  })

  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue
        const el = entry.target as HTMLElement
        const final = parseFloat(el.getAttribute("data-count") ?? "0")
        const duration = 800
        const start = performance.now()
        const tick = (now: number) => {
          const progress = Math.min((now - start) / duration, 1)
          const eased = 1 - Math.pow(1 - progress, 3)
          el.textContent = format(el, final * eased)
          if (progress < 1) requestAnimationFrame(tick)
          else el.textContent = format(el, final)
        }
        requestAnimationFrame(tick)
        observer.unobserve(el)
      }
    },
    { threshold: 0.4 },
  )
  targets.forEach((el) => observer.observe(el))
}