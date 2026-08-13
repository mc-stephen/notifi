export {}

/** Marks the document as JS-capable so CSS-only hidden states can be applied. */
document.documentElement.classList.add("js")

/**
 * Announcement banner: dismiss once per session.
 * Progressive enhancement — without JS the banner simply stays visible.
 */
const banner = document.getElementById("announcement-banner")
if (banner) {
  try {
    if (sessionStorage.getItem("notifi-banner-dismissed") === "1") {
      banner.hidden = true
    }
  } catch {
    /* storage unavailable */
  }
  document.getElementById("announcement-close")?.addEventListener("click", () => {
    banner.hidden = true
    try {
      sessionStorage.setItem("notifi-banner-dismissed", "1")
    } catch {
      /* storage unavailable */
    }
  })
}

/**
 * Header scroll state: adds a stronger border when the page is scrolled.
 */
const header = document.querySelector<HTMLElement>(".site-header")
if (header) {
  const onScroll = () => {
    header.classList.toggle("is-scrolled", window.scrollY > 8)
  }
  onScroll()
  window.addEventListener("scroll", onScroll, { passive: true })
}

/**
 * Mobile navigation drawer toggle.
 */
const drawerToggle = document.getElementById("nav-drawer-toggle")
const drawer = document.getElementById("nav-drawer")
if (drawerToggle && drawer) {
  const open = () => {
    drawer.classList.add("is-open")
    drawerToggle.setAttribute("aria-expanded", "true")
    document.body.classList.add("no-scroll")
  }
  const close = () => {
    drawer.classList.remove("is-open")
    drawerToggle.setAttribute("aria-expanded", "false")
    document.body.classList.remove("no-scroll")
  }
  drawerToggle.addEventListener("click", () => {
    drawer.classList.contains("is-open") ? close() : open()
  })
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") close()
  })
  drawer.querySelectorAll("a").forEach((link) => link.addEventListener("click", close))
}

/**
 * Mega menu close on outside click or Escape.
 * Opening is handled by CSS hover/focus; this only closes.
 */
document.addEventListener("click", (event) => {
  const target = event.target as HTMLElement
  if (!target.closest("[data-mega]")) {
    document.querySelectorAll<HTMLElement>("[data-mega].is-open").forEach((menu) => {
      menu.classList.remove("is-open")
      const trigger = document.querySelector<HTMLElement>(`[aria-controls="${menu.id}"]`)
      trigger?.setAttribute("aria-expanded", "false")
    })
  }
})
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return
  document.querySelectorAll<HTMLElement>("[data-mega].is-open").forEach((menu) => {
    menu.classList.remove("is-open")
    const trigger = document.querySelector<HTMLElement>(`[aria-controls="${menu.id}"]`)
    trigger?.setAttribute("aria-expanded", "false")
  })
})

/**
 * Scroll-reveal: adds .is-visible when an element enters the viewport.
 * Respects reduced motion (CSS side) and never removes the element.
 */
const revealTargets = document.querySelectorAll<HTMLElement>("[data-reveal]")
if (revealTargets.length && "IntersectionObserver" in window) {
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          const el = entry.target as HTMLElement
          const delay = el.dataset.revealDelay
          if (delay) el.style.setProperty("--reveal-delay", `${delay}ms`)
          el.classList.add("is-visible")
          observer.unobserve(el)
        }
      }
    },
    { threshold: 0.12 },
  )
  revealTargets.forEach((el) => observer.observe(el))
}

/**
 * Mega menus: keep aria-expanded in sync with hover/focus opening
 * (closing is handled by the outside-click/Escape handlers above).
 */
document.querySelectorAll<HTMLElement>("[data-mega]").forEach((menu) => {
  const trigger = document.querySelector<HTMLElement>(`[aria-controls="${menu.id}"]`)
  if (!trigger) return
  const sync = () => {
    const open = menu.matches(".is-open") || menu.matches(":hover") || menu.contains(document.activeElement)
    trigger.setAttribute("aria-expanded", String(open))
  }
  menu.addEventListener("mouseenter", sync)
  menu.addEventListener("mouseleave", sync)
  menu.addEventListener("focusin", sync)
  menu.addEventListener("focusout", sync)
  trigger.addEventListener("mouseenter", sync)
  trigger.addEventListener("focus", sync)
})
