const STATUS_URL = "https://status.notifi.dev/api/status.json";

type StatusState = "operational" | "degraded" | "partial_outage" | "major_outage" | "maintenance";

const LABELS: Record<StatusState, string> = {
  operational: "All systems operational",
  degraded: "Performance degraded",
  partial_outage: "Partial outage",
  major_outage: "Major outage",
  maintenance: "Maintenance scheduled",
};

const pill = document.querySelector<HTMLElement>("[data-status-pill]");
if (pill?.dataset.status === "live") {
  const dot = pill.querySelector<HTMLElement>("[data-status-dot]");
  const text = pill.querySelector("[data-status-text]");

  const apply = (state: StatusState) => {
    pill.setAttribute("data-status-state", state);
    if (text) text.textContent = LABELS[state] ?? LABELS.operational;
    if (dot) {
      dot.dataset.state = state;
    }
  };

  fetch(STATUS_URL, { signal: AbortSignal.timeout(6000) })
    .then((res) => (res.ok ? res.json() : Promise.reject(new Error(String(res.status)))))
    .then((data) => {
      if (typeof data?.state === "string") {
        apply(data.state);
      }
    })
    .catch(() => {
      /* keep static fallback snapshot */
    });
}