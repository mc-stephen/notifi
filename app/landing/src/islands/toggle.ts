const STORAGE_KEY = "notifi-billing-cycle";

const toggle = document.querySelector<HTMLButtonElement>("[data-cycle-toggle]");
const labels = document.querySelectorAll<HTMLSpanElement>("[data-cycle-label]");
const priceEls = document.querySelectorAll<HTMLElement>("[data-price]");

function applyCycle(annual: boolean) {
  document.documentElement.dataset.billing = annual ? "annual" : "monthly";
  priceEls.forEach((el) => {
    const monthly = Number(el.dataset.price);
    if (!Number.isFinite(monthly)) return;
    const value = annual ? Math.round(monthly * 0.8) : monthly;
    el.textContent = `$${value.toLocaleString("en-US")}`;
  });
  labels.forEach((label) => {
    label.dataset.active = String(annual ? label.dataset.annual === "true" : label.dataset.annual === "false");
  });
  localStorage.setItem(STORAGE_KEY, annual ? "annual" : "monthly");
}

const saved = localStorage.getItem(STORAGE_KEY);
applyCycle(saved === "annual");
toggle?.addEventListener("click", () => {
  const annual = document.documentElement.dataset.billing !== "annual";
  applyCycle(annual);
});
