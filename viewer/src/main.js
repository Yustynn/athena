import "./style.css";

const POLL_MS = 3000;

const EMPTY_STATE = {
  purpose: null,
  packet: null,
  all_fragments: [],
  feedback_event: null,
  feedback_fragments: [],
  recent_feedback: [],
  scores: { total: 0 },
  fragment_scores: {},
  session: {},
  meta: {
    polled_at: null,
    repo_root: "",
  },
};

const verdictTone = {
  helped: "bg-emerald-400/15 text-emerald-100 ring-emerald-300/20",
  neutral: "bg-stone-400/15 text-stone-100 ring-stone-300/20",
  wrong: "bg-rose-400/15 text-rose-100 ring-rose-300/20",
  redundant: "bg-violet-400/15 text-violet-100 ring-violet-300/20",
  late: "bg-amber-400/15 text-amber-100 ring-amber-300/20",
};

const fragmentKindTone = {
  doctrine: "text-cyan-200",
  pitfall: "text-amber-200",
  procedure: "text-emerald-200",
};

const app = document.querySelector("#app");

let currentState = EMPTY_STATE;
let lastError = "";
let selectedFragmentId = null;
let hasLiveState = false;

function compactTime(isoString) {
  const date = new Date(isoString);
  if (Number.isNaN(date.getTime())) {
    return "unknown";
  }

  return new Intl.DateTimeFormat("en-SG", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(date);
}

function computeScoreRows(scores) {
  return Object.entries(scores)
    .filter(([key, value]) => key !== "total" && value)
    .sort((a, b) => b[1] - a[1]);
}

function fragmentHistoryRows(fragmentId) {
  return (currentState.recent_feedback ?? []).filter(
    (entry) => entry.fragment_id === fragmentId,
  );
}

function scoreClass(score) {
  if (score > 0) return "score-chip score-good";
  if (score < 0) return "score-chip score-bad";
  return "score-chip";
}

function openFragmentModal(fragmentId) {
  selectedFragmentId = fragmentId;
  render();
}

function closeFragmentModal() {
  selectedFragmentId = null;
  render();
}

function attachEvents() {
  document.querySelectorAll("[data-fragment-open]").forEach((node) => {
    node.addEventListener("click", () => {
      openFragmentModal(node.dataset.fragmentOpen);
    });
  });

  document.querySelectorAll("[data-close-modal]").forEach((node) => {
    node.addEventListener("click", closeFragmentModal);
  });

  const overlay = document.querySelector("[data-modal-overlay]");
  if (overlay) {
    overlay.addEventListener("click", (event) => {
      if (event.target === overlay) closeFragmentModal();
    });
  }
}

function render() {
  const state = currentState;
  const packetFragments = state.packet?.fragments ?? [];
  const fragments = state.all_fragments ?? [];
  const feedbackRows = state.feedback_fragments ?? [];
  const recentFeedback = state.recent_feedback ?? [];
  const scoreRows = computeScoreRows(state.scores ?? {});
  const polledAt = compactTime(state.meta?.polled_at);
  const modalRows = selectedFragmentId ? fragmentHistoryRows(selectedFragmentId) : [];
  const modalFragment = selectedFragmentId
    ? fragments.find((fragment) => fragment.fragment_id === selectedFragmentId)
    : null;
  const liveBadge = state.meta?.live
    ? `live ${polledAt}`
    : lastError
      ? "api error"
      : "loading";

  app.innerHTML = `
    <main class="min-h-screen bg-grid text-stone-100">
      <section class="shell">
        <header class="toolbar">
          <div class="toolbar-block">
            <div class="badge badge-cyan">ATHENA</div>
            <div class="toolbar-copy">
              <h1>memory inspector / feedback loop / packet debug</h1>
              <p>sources: scripts/athena current + dolt feedback_events. poll every ${POLL_MS / 1000}s.</p>
            </div>
          </div>
          <div class="toolbar-block toolbar-block-right">
            <div class="badge">${state.purpose?.status ?? "no purpose"}</div>
            <div class="badge">${fragments.length} fragments</div>
            <div class="badge">${packetFragments.length} packet</div>
            <div class="badge">${feedbackRows.length} feedback</div>
            <div class="badge badge-cyan">fragment_nodes</div>
            <div class="badge badge-cyan">dolt feedback</div>
            <div class="badge ${lastError ? "badge-warn" : "badge-cyan"}">${liveBadge}</div>
          </div>
        </header>

        <section class="dashboard-grid">
          <aside class="rail">
            <article class="panel">
              <div class="panel-head">
                <span class="panel-kicker">Purpose</span>
                <span class="tiny-id">${state.purpose?.purpose_id ?? "none"}</span>
              </div>
              <h2 class="panel-title">${state.purpose?.statement ?? "No active Athena purpose in session"}</h2>
              <p class="panel-copy">${
                state.purpose?.success_criteria ??
                (lastError
                  ? `Live read failed: ${lastError}`
                  : "Waiting for live Athena state.")
              }</p>
              <dl class="kv-table">
                <div class="kv-row"><dt>packet</dt><dd>${state.packet?.packet_id ?? "none"}</dd></div>
                <div class="kv-row"><dt>session purpose</dt><dd>${state.session?.purpose_id ?? "none"}</dd></div>
                <div class="kv-row"><dt>session packet</dt><dd>${state.session?.packet_id ?? "none"}</dd></div>
                <div class="kv-row"><dt>repo root</dt><dd>${state.meta?.repo_root ?? "unknown"}</dd></div>
              </dl>
            </article>

            <article class="panel">
              <div class="panel-head">
                <span class="panel-kicker">Packet</span>
                <span class="tiny-id">${state.packet?.packet_id ?? "none"}</span>
              </div>
              <div class="list-panel">
                <div class="summary-row">
                  <span>packet fragments</span>
                  <strong>${packetFragments.length}</strong>
                </div>
                <div class="summary-row">
                  <span>store fragments</span>
                  <strong>${fragments.length}</strong>
                </div>
                <div class="summary-row">
                  <span>feedback event</span>
                  <strong>${state.feedback_event?.outcome ?? "none"}</strong>
                </div>
                <div class="summary-row">
                  <span>feedback packet</span>
                  <strong>${state.feedback_event?.packet_id ?? "none"}</strong>
                </div>
                <div class="summary-row">
                  <span>last live poll</span>
                  <strong>${polledAt}</strong>
                </div>
              </div>
            </article>
          </aside>

          <section class="panel panel-main">
            <div class="panel-head">
              <div>
                <span class="panel-kicker">Fragments</span>
                <h2 class="panel-title">all stored fragments</h2>
              </div>
              <div class="dense-meta">
                <span>id</span>
                <span>summary</span>
                <span>feedback</span>
              </div>
            </div>

            <div class="fragment-list">
              ${
                fragments.length
                  ? fragments
                .map((fragment) => {
                  const feedback = feedbackRows.find(
                    (entry) => entry.fragment_id === fragment.fragment_id,
                  );
                  const fragmentScore = state.fragment_scores?.[fragment.fragment_id] ?? 0;
                  const inPacket = packetFragments.some(
                    (packetFragment) => packetFragment.fragment_id === fragment.fragment_id,
                  );

                  return `
                    <article class="fragment-row" data-fragment-open="${fragment.fragment_id}">
                      <div class="fragment-meta">
                        <div class="fragment-id">${fragment.fragment_id}</div>
                        <div class="fragment-kind ${fragmentKindTone[fragment.kind] ?? ""}">${fragment.kind}</div>
                        <div class="${scoreClass(fragmentScore)}">score ${fragmentScore}</div>
                        ${inPacket ? `<div class="badge badge-cyan">in packet</div>` : ""}
                      </div>
                      <div class="fragment-content">
                        <h3>${fragment.summary}</h3>
                        <p>${fragment.full_text}</p>
                      </div>
                      <div class="fragment-feedback">
                        ${
                          feedback
                            ? `<span class="rounded-full px-2 py-1 text-[10px] uppercase tracking-[0.22em] ring-1 ${verdictTone[feedback.verdict] ?? verdictTone.neutral}">${feedback.verdict}</span>
                               <p>${feedback.reason}</p>`
                            : `<span class="rounded-full px-2 py-1 text-[10px] uppercase tracking-[0.22em] ring-1 ${verdictTone.late}">missing</span>
                               <p>No feedback row for fragment.</p>`
                        }
                      </div>
                    </article>
                  `;
                })
                .join("")
                  : `<div class="empty-row">No stored fragments in Athena memory.</div>`
              }
            </div>
          </section>

          <aside class="rail">
            <article class="panel">
              <div class="panel-head">
                <span class="panel-kicker">Scores</span>
                <span class="tiny-id">${state.scores?.total ?? 0} rows</span>
              </div>
              <div class="score-list">
                ${
                  scoreRows.length
                    ? scoreRows
                        .map(
                          ([key, value]) => `
                            <div class="summary-row">
                              <span>${key}</span>
                              <strong>${value}</strong>
                            </div>
                          `,
                        )
                        .join("")
                    : `<div class="empty-row">No score rows yet.</div>`
                }
              </div>
            </article>

            <article class="panel">
              <div class="panel-head">
                <span class="panel-kicker">Feedback Loop</span>
                <span class="tiny-id">${state.feedback_event?.feedback_id ?? "none"}</span>
              </div>
              <div class="feedback-list">
                ${feedbackRows
                  .map(
                    (entry) => `
                      <div class="feedback-row">
                        <div class="flex items-center justify-between gap-2">
                          <span class="fragment-id">${entry.fragment_id}</span>
                          <span class="rounded-full px-2 py-1 text-[10px] uppercase tracking-[0.22em] ring-1 ${verdictTone[entry.verdict] ?? verdictTone.neutral}">${entry.verdict}</span>
                        </div>
                        <p>${entry.reason || "No reason."}</p>
                      </div>
                    `,
                  )
                  .join("")}
              </div>
            </article>

            <article class="panel">
              <div class="panel-head">
                <span class="panel-kicker">Recent Feedback</span>
                <span class="tiny-id">${recentFeedback.length} rows</span>
              </div>
              <div class="feedback-list">
                ${recentFeedback
                  .slice(0, 8)
                  .map(
                    (entry) => `
                      <button class="history-row" data-fragment-open="${entry.fragment_id}">
                        <div class="flex items-center justify-between gap-2">
                          <span class="fragment-id">${entry.fragment_id}</span>
                          <span class="${scoreClass(state.fragment_scores?.[entry.fragment_id] ?? 0)}">score ${state.fragment_scores?.[entry.fragment_id] ?? 0}</span>
                        </div>
                        <div class="history-meta">
                          <span>${entry.verdict}</span>
                          <span>${entry.outcome}</span>
                          <span>${entry.packet_id}</span>
                        </div>
                        <p>${entry.reason || "No reason."}</p>
                      </button>
                    `,
                  )
                  .join("")}
              </div>
            </article>

            <article class="panel">
              <div class="panel-head">
                <span class="panel-kicker">Step View</span>
                <span class="tiny-id">read-only</span>
              </div>
              <div class="step-list">
                <div class="step-row is-active">1. memory state</div>
                <div class="step-row">2. feedback event</div>
                <div class="step-row">3. next packet debug</div>
              </div>
              ${
                lastError
                  ? `<p class="error-copy">${lastError}</p>`
                  : `<p class="panel-copy mt-3">Dev server reads Athena state from repo and polls automatically.</p>`
              }
            </article>
          </aside>
        </section>

        ${
          modalFragment
            ? `
              <div class="modal-overlay" data-modal-overlay>
                <div class="modal-card">
                  <div class="panel-head">
                    <div>
                      <div class="panel-kicker">Fragment Feedback History</div>
                      <h2 class="panel-title">${modalFragment.fragment_id} / ${modalFragment.summary}</h2>
                    </div>
                    <button class="modal-close" data-close-modal>close</button>
                  </div>
                  <p class="panel-copy">Recent feedback rows for selected fragment. Newest first.</p>
                  <div class="modal-list">
                    ${modalRows
                      .map(
                        (entry) => `
                          <div class="modal-row">
                            <div class="modal-row-top">
                              <span class="rounded-full px-2 py-1 text-[10px] uppercase tracking-[0.22em] ring-1 ${verdictTone[entry.verdict] ?? verdictTone.neutral}">${entry.verdict}</span>
                              <span>${entry.outcome}</span>
                              <span>${entry.packet_id}</span>
                              <span>${entry.feedback_id}</span>
                            </div>
                            <p>${entry.reason || "No reason."}</p>
                          </div>
                        `,
                      )
                      .join("")}
                  </div>
                </div>
              </div>
            `
            : ""
        }
      </section>
    </main>
  `;

  attachEvents();
}

async function loadState() {
  try {
    const response = await fetch("/api/athena/state", { cache: "no-store" });
    if (!response.ok) {
      throw new Error(`API ${response.status}`);
    }

    currentState = await response.json();
    lastError = currentState.meta?.error ?? "";
    hasLiveState = Boolean(currentState.meta?.live);
  } catch (error) {
    lastError = error instanceof Error ? error.message : String(error);
    if (!hasLiveState) {
      currentState = EMPTY_STATE;
    }
  }

  render();
}

render();
loadState();
setInterval(loadState, POLL_MS);

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && selectedFragmentId) {
    closeFragmentModal();
  }
});
