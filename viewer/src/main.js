import "./style.css";

const session = {
  workspace: "athena-v2/.athena/db",
  purpose: {
    id: "purpose-1776015730378746000",
    statement: "Build condensed internal Athena viewer with hot reload",
    successCriteria:
      "Compact Tailwind scaffold for internal inspection and fast iteration.",
    status: "active",
    owner: "repo session",
    updatedAt: "2026-04-13 02:12 SGT",
  },
  packet: {
    id: "packet-1776017177730532000",
    scope: "assemble_packet",
    summary: "Packet view for purpose, fragments, verdicts, and commit gates.",
    coverage: "3 fragments / 2 verdict gaps / dev scaffold",
    stage: "review",
  },
  fragments: [
    {
      id: "f1",
      kind: "doctrine",
      summary: "Keep runtime deterministic.",
      body:
        "Prefer packet assembly, feedback scoring, and persistence flows that produce stable results from same inputs.",
      signal: "high confidence",
      state: "selected",
    },
    {
      id: "f2",
      kind: "pitfall",
      summary: "Do not skip fragment feedback.",
      body:
        "Every fragment sent in packet needs explicit verdict so retrieval quality changes stay grounded in evidence.",
      signal: "needs review",
      state: "pending verdict",
    },
    {
      id: "f3",
      kind: "procedure",
      summary: "Validate packet before commit.",
      body:
        "Check packet scope, fragment selection, and coverage before persisting changes into Athena memory.",
      signal: "healthy",
      state: "selected",
    },
  ],
  feedback: [
    {
      fragmentId: "f1",
      verdict: "keep",
      rationale: "Supports reliable debugging and repeatable tests.",
    },
    {
      fragmentId: "f2",
      verdict: "missing",
      rationale: "Packet shipped without explicit fragment verdict yet.",
    },
    {
      fragmentId: "f3",
      verdict: "reinforce",
      rationale: "Viewer needs visible commit gate before storage write.",
    },
  ],
  checkpoints: [
    "purpose aligned",
    "packet assembled",
    "feedback applied",
    "commit gate visible",
  ],
};

const verdictTone = {
  keep: "bg-emerald-400/15 text-emerald-100 ring-emerald-300/20",
  missing: "bg-amber-400/15 text-amber-100 ring-amber-300/20",
  reinforce: "bg-sky-400/15 text-sky-100 ring-sky-300/20",
};

const fragmentTone = {
  selected: "border-white/10 bg-white/[0.035]",
  "pending verdict": "border-amber-300/30 bg-amber-300/[0.06]",
};

document.querySelector("#app").innerHTML = `
  <main class="min-h-screen bg-grid text-stone-100">
    <section class="shell">
      <header class="toolbar">
        <div class="flex min-w-0 items-center gap-3">
          <div class="badge badge-cyan">ATHENA</div>
          <div class="min-w-0">
            <h1 class="truncate text-sm font-semibold uppercase tracking-[0.22em] text-white">viewer / dev shell</h1>
            <p class="truncate text-[11px] text-stone-400">${session.packet.summary}</p>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <div class="badge">${session.purpose.status}</div>
          <div class="badge">${session.packet.stage}</div>
          <div class="badge">${session.fragments.length} fragments</div>
          <div class="badge">vite hmr</div>
        </div>
      </header>

      <section class="dashboard-grid">
        <aside class="rail">
          <article class="panel">
            <div class="panel-head">
              <span class="panel-kicker">Purpose</span>
              <span class="tiny-id">${session.purpose.id}</span>
            </div>
            <h2 class="panel-title">${session.purpose.statement}</h2>
            <p class="panel-copy">${session.purpose.successCriteria}</p>
            <dl class="kv-table">
              <div class="kv-row"><dt>owner</dt><dd>${session.purpose.owner}</dd></div>
              <div class="kv-row"><dt>updated</dt><dd>${session.purpose.updatedAt}</dd></div>
              <div class="kv-row"><dt>workspace</dt><dd>${session.workspace}</dd></div>
              <div class="kv-row"><dt>packet</dt><dd>${session.packet.id}</dd></div>
            </dl>
          </article>

          <article class="panel">
            <div class="panel-head">
              <span class="panel-kicker">Checks</span>
              <span class="tiny-id">${session.checkpoints.length} gates</span>
            </div>
            <div class="list-panel">
              ${session.checkpoints
                .map(
                  (checkpoint, index) => `
                    <div class="check-row">
                      <span class="check-index">${index + 1}</span>
                      <span>${checkpoint}</span>
                    </div>
                  `,
                )
                .join("")}
            </div>
          </article>
        </aside>

        <section class="panel panel-main">
          <div class="panel-head">
            <div>
              <span class="panel-kicker">Packet</span>
              <h2 class="panel-title mt-1">${session.packet.coverage}</h2>
            </div>
            <div class="flex flex-wrap gap-2">
              <div class="badge badge-cyan">${session.packet.scope}</div>
              <div class="badge">${session.packet.stage}</div>
            </div>
          </div>

          <div class="dense-header">
            <span>fragment</span>
            <span>summary</span>
            <span>signal</span>
          </div>

          <div class="fragment-list">
            ${session.fragments
              .map(
                (fragment) => `
                  <article class="fragment-row ${fragmentTone[fragment.state] ?? fragmentTone.selected}">
                    <div class="fragment-meta">
                      <div class="fragment-id">${fragment.id}</div>
                      <div class="fragment-kind">${fragment.kind}</div>
                      <div class="badge">${fragment.state}</div>
                    </div>
                    <div class="fragment-content">
                      <h3>${fragment.summary}</h3>
                      <p>${fragment.body}</p>
                    </div>
                    <div class="fragment-signal">${fragment.signal}</div>
                  </article>
                `,
              )
              .join("")}
          </div>
        </section>

        <aside class="rail">
          <article class="panel">
            <div class="panel-head">
              <span class="panel-kicker">Feedback</span>
              <span class="tiny-id">${session.feedback.length} entries</span>
            </div>
            <div class="feedback-list">
              ${session.feedback
                .map(
                  (entry) => `
                    <div class="feedback-row">
                      <div class="flex items-center justify-between gap-2">
                        <span class="fragment-id">${entry.fragmentId}</span>
                        <span class="rounded-full px-2 py-1 text-[10px] uppercase tracking-[0.22em] ring-1 ${verdictTone[entry.verdict]}">${entry.verdict}</span>
                      </div>
                      <p>${entry.rationale}</p>
                    </div>
                  `,
                )
                .join("")}
            </div>
          </article>

          <article class="panel">
            <div class="panel-head">
              <span class="panel-kicker">Dev Loop</span>
              <span class="tiny-id">hot reload</span>
            </div>
            <div class="command-list">
              <code>cd viewer</code>
              <code>npm install</code>
              <code>npm run dev</code>
            </div>
            <p class="panel-copy mt-3">Edit index.html, src/main.js, src/style.css. Vite pushes refresh into browser.</p>
          </article>
        </aside>
      </section>
    </section>
  </main>
`;
