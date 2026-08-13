export {}

/** Copy-to-clipboard for code windows. Clipboard API only — HTTPS required. */
const buttons = document.querySelectorAll<HTMLButtonElement>("[data-copy]")
buttons.forEach((button) => {
  const label = button.querySelector("[data-copy-label]")
  const setLabel = (text: string) => {
    if (!label) return
    const original = label.textContent
    label.textContent = text
    window.setTimeout(() => {
      label.textContent = original
    }, 1500)
  }
  button.addEventListener("click", async () => {
    const code = button.getAttribute("data-copy") ?? ""
    try {
      await navigator.clipboard.writeText(code)
      setLabel("Copied")
    } catch {
      setLabel("Press Cmd/Ctrl+C")
    }
  })
})
