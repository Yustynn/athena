import "./style.css";

const POLL_MS = 3000;

const EMPTY_STATE = {
  purpose: null,
  packet: null,
  packet_history: [],
  all_fragments: [],
  feedback_event: null,
  feedback_events: [],
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

const outcomeTone = {
  success: "bg-emerald-400/15 text-emerald-100 ring-emerald-300/20",
  partial: "bg-cyan-400/15 text-cyan-100 ring-cyan-300/20",
  failed: "bg-rose-400/15 text-rose-100 ring-rose-300/20",
  abandoned: "bg-amber-400/15 text-amber-100 ring-amber-300/20",
  superseded: "bg-violet-400/15 text-violet-100 ring-violet-300/20",
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
let selectedPacketId = null;
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

function statusPill(label, toneClass) {
  return `<span class="rounded-full px-2 py-1 text-[10px] uppercase tracking-[0.22em] ring-1 ${toneClass}">${label}</span>`;
}

function selectedPacket(state) {
  const packetHistory = state.packet_history ?? [];

  return (
    packetHistory.find((packet) => packet.packet_id === selectedPacketId) ??
    packetHistory.find((packet) => packet.packet_id === state.session?.packet_id) ??
    state.packet ??
    packetHistory[0] ??
    null
  );
}

function syncSelectedPacket(state) {
  const packetHistory = state.packet_history ?? [];
  const packetIds = packetHistory.map((packet) => packet.packet_id);

  if (!packetIds.length) {
    selectedPacketId = null;
    return;
  }

  if (selectedPacketId && packetIds.includes(selectedPacketId)) {
    return;
  }

  selectedPacketId =
    state.session?.packet_id && packetIds.includes(state.session.packet_id)
      ? state.session.packet_id
      : state.packet?.packet_id ?? packetIds[0];
}

function feedbackEventForPacket(state, packetId) {
  return (state.feedback_events ?? []).find((entry) => entry.packet_id === packetId) ?? null;
}

function feedbackRowsForPacket(state, packetId) {
  const event = feedbackEventForPacket(state, packetId);

  if (!event) {
    return [];
  }

  return (state.recent_feedback ?? []).filter(
    (entry) => entry.feedback_id === event.feedback_id,
  );
}

function findFragment(state, fragmentId) {
  return (
    (state.all_fragments ?? []).find((fragment) => fragment.fragment_id === fragmentId) ??
    (state.packet_history ?? [])
      .flatMap((packet) => packet.fragments ?? [])
      .find((fragment) => fragment.fragment_id === fragmentId) ??
    null
  );
}

function renderFragmentRows(fragments, feedbackRows, activePacketId) {
  const feedbackByFragmentId = new Map(
    feedbackRows.map((entry) => [entry.fragment_id, entry]),
  );
  const packetFragmentIds = new Set(
    ((currentState.packet_history ?? []).find(
      (packet) => packet.packet_id === activePacketId,
    )?.fragments ?? []
    ).map((fragment) => fragment.fragment_id),
  );

  if (!fragments.length) {
    return `<div class="empty-row">No fragments available.</div>`;
  }

  return fragments
    .map((fragment) => {
      const feedback = feedbackByFragmentId.get(fragment.fragment_id);
      const fragmentScore = currentState.fragment_scores?.[fragment.fragment_id] ?? 0;
      const inSelectedPacket = packetFragmentIds.has(fragment.fragment_id);

      return `
        <article class="fragment-row" data-fragment-open="${fragment.fragment_id}">
          <div class="fragment-meta">
            <div class="fragment-id">${fragment.fragment_id}</div>
            <div class="fragment-kind ${fragmentKindTone[fragment.kind] ?? ""}">${fragment.kind}</div>
            <div class="${scoreClass(fragmentScore)}">score ${fragmentScore}</div>
            ${inSelectedPacket ? `<div class="badge badge-cyan">selected packet</div>` : ""}
          </div>
          <div class="fragment-content">
            <h3>${fragment.summary ?? fragment.full_text ?? fragment.fragment_id}</h3>
            <p>${fragment.full_text ?? fragment.summary ?? "No fragment body."}</p>
          </div>
          <div class="fragment-feedback">
            ${
              feedback
                ? `${statusPill(feedback.verdict, verdictTone[feedback.verdict] ?? verdictTone.neutral)}
                   <p>${feedback.reason || "No reason."}</p>`
                : `${statusPill("missing", verdictTone.late)}
                   <p>No feedback row for fragment.</p>`
            }
          </div>
        </article>
      `;
    })
    .join("");
}

function openFragmentModal(fragmentId) {
  selectedFragmentId = fragmentId;
  render();
}

function closeFragmentModal() {
  selectedFragmentId = null;
  render();
}

function selectPacket(packetId) {
  selectedPacketId = packetId;
  render();
}

function attachEvents() {
  document.querySelectorAll("[data-fragment-open]").forEach((node) => {
    node.addEventListener("click", () => {
      openFragmentModal(node.dataset.fragmentOpen);
    });
  });

  document.querySelectorAll("[data-packet-select]").forEach((node) => {
    node.addEventListener("click", () => {
      selectPacket(node.dataset.packetSelect);
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
  syncSelectedPacket(state);

  const packetHistory = state.packet_history ?? [];
  const activePacket = selectedPacket(state);
  const activePacketFragments = activePacket?.fragments ?? [];
  const activePacketFeedback = activePacket
    ? feedbackRowsForPacket(state, activePacket.packet_id)
    : [];
  const activePacketEvent = activePacket
    ? feedbackEventForPacket(state, activePacket.packet_id)
    : null;
  const fragments = state.all_fragments ?? [];
  const feedbackRows = state.feedback_fragments ?? [];
  const feedbackEvents = state.feedback_events ?? [];
  const recentFeedback = state.recent_feedback ?? [];
  const scoreRows = computeScoreRows(state.scores ?? {});
  const polledAt = compactTime(state.meta?.polled_at);
  const modalRows = selectedFragmentId ? fragmentHistoryRows(selectedFragmentId) : [];
  const modalFragment = selectedFragmentId ? findFragment(state, selectedFragmentId) : null;
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
              <p>sources: packet history + feedback events from dolt. poll every ${POLL_MS / 1000}s.</p>
            </div>
          </div>
          <div class="toolbar-block toolbar-block-right">
            <div class="badge">${state.purpose?.status ?? "no purpose"}</div>
            <div class="badge">${fragments.length} store</div>
            <div class="badge">${packetHistory.length} packets</div>
            <div class="badge">${feedbackEvents.length} events</div>
            <div class="badge badge-cyan">packet timeline</div>
            <div class="badge badge-cyan">coverage stats</div>
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
                <div class="kv-row"><dt>active packet</dt><dd>${activePacket?.packet_id ?? "none"}</dd></div>
                <div class="kv-row"><dt>session purpose</dt><dd>${state.session?.purpose_id ?? "none"}</dd></div>
                <div class="kv-row"><dt>session packet</dt><dd>${state.session?.packet_id ?? "none"}</dd></div>
                <div class="kv-row"><dt>repo root</dt><dd>${state.meta?.repo_root ?? "unknown"}</dd></div>
              </dl>
            </article>

            <article class="panel">
              <div class="panel-head">
                <span class="panel-kicker">Packet Timeline</span>
                <span class="tiny-id">${packetHistory.length} versions</span>
              </div>
              <div class="timeline-list">
                ${
                  packetHistory.length
                    ? packetHistory
                        .map((packet, index) => {
                          const event = feedbackEventForPacket(state, packet.packet_id);
                          const coverageLabel = event
                            ? `${event.fragment_feedback_count}/${event.packet_fragment_count || packet.fragments.length}`
                            : "pending";

                          return `
                            <button class="timeline-row ${packet.packet_id === activePacket?.packet_id ? "is-active" : ""}" data-packet-select="${packet.packet_id}">
                              <div class="timeline-top">
                                <span class="timeline-index">v${packetHistory.length - index}</span>
                                <span class="tiny-id">${packet.packet_id}</span>
                              </div>
                              <div class="timeline-meta">
                                <span>${packet.fragments.length} fragments</span>
                                <span>${event?.outcome ?? "no feedback"}</span>
                                <span>coverage ${coverageLabel}</span>
                              </div>
                            </button>
                          `;
                        })
                        .join("")
                    : `<div class="empty-row">No packet history for purpose.</div>`
                }
              </div>
            </article>
          </aside>

          <section class="panel-stack">
            <section class="panel panel-main">
              <div class="panel-head">
                <div>
                  <span class="panel-kicker">Packet Explorer</span>
                  <h2 class="panel-title">selected packet fragments</h2>
                </div>
                <span class="tiny-id">${activePacket?.packet_id ?? "none"}</span>
              </div>

              <div class="packet-summary-grid">
                <div class="summary-row"><span>packet fragments</span><strong>${activePacketFragments.length}</strong></div>
                <div class="summary-row"><span>feedback outcome</span><strong>${activePacketEvent?.outcome ?? "none"}</strong></div>
                <div class="summary-row"><span>coverage</span><strong>${
                  activePacketEvent
                    ? `${activePacketEvent.fragment_feedback_count}/${activePacketEvent.packet_fragment_count}`
                    : "pending"
                }</strong></div>
                <div class="summary-row"><span>feedback id</span><strong>${activePacketEvent?.feedback_id ?? "none"}</strong></div>
              </div>

              <div class="fragment-list">
                ${renderFragmentRows(activePacketFragments, activePacketFeedback, activePacket?.packet_id)}
              </div>
            </section>

            <section class="panel panel-main">
              <div class="panel-head">
                <div>
                  <span class="panel-kicker">Fragment Store</span>
                  <h2 class="panel-title">all stored fragments</h2>
                </div>
                <div class="dense-meta">
                  <span>id</span>
                  <span>summary</span>
                  <span>feedback</span>
                </div>
              </div>

              <div class="fragment-list">
                ${renderFragmentRows(fragments, feedbackRows, activePacket?.packet_id)}
              </div>
            </section>
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
                <span class="panel-kicker">Feedback Events</span>
                <span class="tiny-id">${feedbackEvents.length} events</span>
              </div>
              <div class="feedback-list">
                ${
                  feedbackEvents.length
                    ? feedbackEvents
                        .map(
                          (entry) => `
                            <div class="feedback-row">
                              <div class="feedback-topline">
                                <span class="fragment-id">${entry.feedback_id}</span>
                                ${statusPill(entry.outcome, outcomeTone[entry.outcome] ?? outcomeTone.partial)}
                              </div>
                              <div class="history-meta">
                                <span>${entry.packet_id}</span>
                                <span>${entry.fragment_feedback_count}/${entry.packet_fragment_count || 0} rows</span>
                                <span>${entry.coverage_complete ? "coverage complete" : "coverage partial"}</span>
                              </div>
                            </div>
                          `,
                        )
                        .join("")
                    : `<div class="empty-row">No feedback events yet.</div>`
                }
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
                <span class="panel-kicker">Doc Gaps</span>
                <span class="tiny-id">next backend slices</span>
              </div>
              <div class="step-list">
                <div class="step-row is-active">packet timeline now visible</div>
                <div class="step-row is-active">feedback coverage now visible</div>
                <div class="step-row is-active">selected packet inspection now visible</div>
                <div class="step-row">orientation_response not persisted yet</div>
                <div class="step-row">correction_packet not persisted yet</div>
              </div>
              ${
                lastError
                  ? `<p class="error-copy">${lastError}</p>`
                  : `<p class="panel-copy mt-3">Design docs mention orientation and correction loop, but current Dolt schema only stores purposes, packets, fragments, and feedback.</p>`
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
                              ${statusPill(entry.verdict, verdictTone[entry.verdict] ?? verdictTone.neutral)}
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
    syncSelectedPacket(currentState);
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
