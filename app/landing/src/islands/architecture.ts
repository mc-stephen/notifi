export {}

/**
 * Architecture diagram: hovering a stage highlights its caption.
 * Pure enhancement — the page is fully understandable without it.
 */
const diagram = document.getElementById("architecture-diagram")
const captions = document.querySelectorAll<HTMLElement>("[data-stage-caption]")
if (diagram && captions.length) {
  const stages = diagram.querySelectorAll<SVGGElement>("[data-stage]")
  const setActive = (index: number | null) => {
    captions.forEach((caption, i) => {
      caption.classList.toggle("is-active", i === index)
    })
  }
  stages.forEach((stage, index) => {
    stage.addEventListener("mouseenter", () => setActive(index))
    stage.addEventListener("mouseleave", () => setActive(null))
    stage.addEventListener("focusin", () => setActive(index))
    stage.addEventListener("focusout", () => setActive(null))
  })
}