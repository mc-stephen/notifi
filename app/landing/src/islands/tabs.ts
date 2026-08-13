export {}

/**
 * Tab panels (interactive API example).
 * Markup carries role=tab / role=tabpanel; without JS all panels show.
 * With JS: only the active panel is visible, arrows navigate.
 */
const tabsets = document.querySelectorAll<HTMLElement>("[data-tabs]")
tabsets.forEach((tabset) => {
  const tabs = Array.from(tabset.querySelectorAll<HTMLButtonElement>('[role="tab"]'))
  const panels = Array.from(tabset.querySelectorAll<HTMLElement>('[role="tabpanel"]'))
  if (!tabs.length || !panels.length) return

  const activate = (index: number) => {
    tabs.forEach((tab, i) => {
      const active = i === index
      tab.setAttribute("aria-selected", String(active))
      tab.tabIndex = active ? 0 : -1
    })
    panels.forEach((panel, i) => {
      panel.hidden = i !== index
    })
  }

  // Progressive enhancement: hide non-first panels only now that JS exists.
  panels.forEach((panel, i) => {
    panel.hidden = i !== 0
  })

  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => activate(index))
    tab.addEventListener("keydown", (event) => {
      const current = tabs.indexOf(document.activeElement as HTMLButtonElement)
      let next: number | null = null
      if (event.key === "ArrowRight") next = (current + 1) % tabs.length
      if (event.key === "ArrowLeft") next = (current - 1 + tabs.length) % tabs.length
      if (event.key === "Home") next = 0
      if (event.key === "End") next = tabs.length - 1
      if (next !== null) {
        event.preventDefault()
        activate(next)
        tabs[next].focus()
      }
    })
  })
})

/**
 * "Run" sequence for the API example: sending… → delivered.
 * Without JS the final state is rendered statically.
 */
const runs = document.querySelectorAll<HTMLButtonElement>("[data-run]")
const runStatus = document.getElementById("api-status")
runs.forEach((button) => {
  button.addEventListener("click", () => {
    if (!runStatus) return
    runStatus.textContent = "sending…"
    runStatus.classList.remove("is-ok")
    window.setTimeout(() => {
      runStatus.textContent = "delivered in 412ms across 3 channels"
      runStatus.classList.add("is-ok")
    }, 900)
  })
})