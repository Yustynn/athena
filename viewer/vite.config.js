import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

const repoRoot = path.resolve(import.meta.dirname, "..");
const athenaDb = path.join(repoRoot, ".athena", "db");
const doltHome = path.join(repoRoot, ".athena", ".dolt-home");
const sessionPath = path.join(repoRoot, ".athena", "session.json");
const COMMAND_TIMEOUT_MS = 1200;
const REFRESH_MS = 2000;

function runDolt(sql) {
  return execFileSync("dolt", ["sql", "-q", sql, "-r", "json"], {
    cwd: athenaDb,
    encoding: "utf8",
    timeout: COMMAND_TIMEOUT_MS,
    env: {
      ...process.env,
      HOME: doltHome,
    },
  });
}

function readSession() {
  if (!fs.existsSync(sessionPath)) {
    return {};
  }

  return JSON.parse(fs.readFileSync(sessionPath, "utf8"));
}

function summarizeScores(feedbackRows) {
  return feedbackRows.reduce(
    (acc, row) => {
      acc.total += 1;
      acc[row.verdict] = (acc[row.verdict] || 0) + 1;
      return acc;
    },
    { total: 0 },
  );
}

function computeFragmentScores(historyRows) {
  return historyRows.reduce((acc, row) => {
    const delta =
      row.verdict === "helped" ? 1 : row.verdict === "wrong" ? -2 : 0;
    acc[row.fragment_id] = (acc[row.fragment_id] || 0) + delta;
    return acc;
  }, {});
}

function runDoltRows(sql) {
  return JSON.parse(runDolt(sql)).rows;
}

function loadAthenaState() {
  const session = readSession();
  const purposeId =
    session.purpose_id ||
    runDoltRows(
      "SELECT purpose_id FROM purposes ORDER BY purpose_id DESC LIMIT 1;",
    )[0]?.purpose_id ||
    null;

  const purpose = purposeId
    ? runDoltRows(
        `SELECT purpose_id, statement, status, success_criteria
         FROM purposes
         WHERE purpose_id = '${purposeId}'
         LIMIT 1;`,
      )[0] ?? null
    : null;

  const packetIdFromSession = session.packet_id || null;
  const packet =
    (packetIdFromSession
      ? runDoltRows(
          `SELECT packet_id, purpose_id
           FROM packets
           WHERE packet_id = '${packetIdFromSession}'
           LIMIT 1;`,
        )[0]
      : null) ||
    (purposeId
      ? runDoltRows(
          `SELECT packet_id, purpose_id
           FROM packets
           WHERE purpose_id = '${purposeId}'
           ORDER BY packet_id DESC
           LIMIT 1;`,
        )[0]
      : null) ||
    null;

  const fragments = packet
    ? runDoltRows(
        `SELECT fragment_id, kind, IFNULL(summary, text) AS summary, IFNULL(full_text, text) AS full_text
         FROM packet_fragments
         WHERE packet_id = '${packet.packet_id}'
         ORDER BY position ASC;`,
      )
    : [];
  const allFragments = runDoltRows(
    `SELECT fragment_id, kind, IFNULL(summary, text) AS summary, IFNULL(full_text, text) AS full_text
     FROM fragment_nodes
     ORDER BY fragment_id ASC;`,
  );

  let feedback = [];
  let feedbackEvent = null;
  let recentFeedback = [];

  if (purposeId) {
    const feedbackEvents = JSON.parse(
      runDolt(
        `SELECT feedback_id, purpose_id, packet_id, outcome
         FROM feedback_events
         WHERE purpose_id = '${purposeId}'
         ORDER BY feedback_id DESC
         LIMIT 1;`,
      ),
    ).rows;

    feedbackEvent = feedbackEvents[0] ?? null;

    if (feedbackEvent) {
      feedback = JSON.parse(
        runDolt(
          `SELECT fragment_id, verdict, IFNULL(reason, '') AS reason
           FROM feedback_fragments
           WHERE feedback_id = '${feedbackEvent.feedback_id}'
           ORDER BY position ASC;`,
        ),
      ).rows;
    }

    recentFeedback = JSON.parse(
      runDolt(
        `SELECT fe.feedback_id, fe.packet_id, fe.outcome, ff.fragment_id, ff.verdict, IFNULL(ff.reason, '') AS reason
         FROM feedback_events fe
         JOIN feedback_fragments ff ON ff.feedback_id = fe.feedback_id
         WHERE fe.purpose_id = '${purposeId}'
         ORDER BY fe.feedback_id DESC, ff.position ASC
         LIMIT 40;`,
      ),
    ).rows;
  }

  return {
    session,
    purpose,
    packet: packet
      ? {
          ...packet,
          fragments,
        }
      : null,
    all_fragments: allFragments,
    feedback_event: feedbackEvent,
    feedback_fragments: feedback,
    recent_feedback: recentFeedback,
    scores: summarizeScores(feedback),
    fragment_scores: computeFragmentScores(recentFeedback),
    meta: {
      polled_at: new Date().toISOString(),
      repo_root: repoRoot,
    },
  };
}

function athenaApiPlugin() {
  let cache = {
    session: {},
    purpose: null,
    packet: null,
    feedback_event: null,
    feedback_fragments: [],
    recent_feedback: [],
    scores: { total: 0 },
    fragment_scores: {},
    meta: {
      polled_at: null,
      repo_root: repoRoot,
      live: false,
      error: "waiting for first refresh",
    },
  };
  let refreshTimer = null;
  let refreshInFlight = false;

  function refreshCache() {
    if (refreshInFlight) {
      return;
    }

    refreshInFlight = true;
    try {
      const next = loadAthenaState();
      cache = {
        ...next,
        meta: {
          ...next.meta,
          live: true,
          error: null,
        },
      };
    } catch (error) {
      cache = {
        ...cache,
        meta: {
          ...cache.meta,
          polled_at: new Date().toISOString(),
          live: false,
          error: error instanceof Error ? error.message : String(error),
        },
      };
    } finally {
      refreshInFlight = false;
    }
  }

  return {
    name: "athena-api",
    configureServer(server) {
      refreshCache();
      refreshTimer = setInterval(refreshCache, REFRESH_MS);
      server.httpServer?.once("close", () => {
        if (refreshTimer) clearInterval(refreshTimer);
      });

      server.middlewares.use("/api/athena/state", (_req, res) => {
        const body = JSON.stringify(cache);
        res.setHeader("Content-Type", "application/json");
        res.end(body);
      });
    },
  };
}

export default defineConfig({
  plugins: [tailwindcss(), athenaApiPlugin()],
});
