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

function groupPacketHistory(packetRows) {
  const packets = new Map();

  for (const row of packetRows) {
    if (!packets.has(row.packet_id)) {
      packets.set(row.packet_id, {
        packet_id: row.packet_id,
        purpose_id: row.purpose_id,
        fragments: [],
      });
    }

    if (row.fragment_id) {
      packets.get(row.packet_id).fragments.push({
        fragment_id: row.fragment_id,
        kind: row.kind,
        summary: row.summary,
        full_text: row.full_text,
        position: row.position,
      });
    }
  }

  return Array.from(packets.values());
}

function enrichFeedbackEvents(feedbackEvents, packetHistory) {
  return feedbackEvents.map((entry) => {
    const packetMatch = packetHistory.find(
      (packetEntry) => packetEntry.packet_id === entry.packet_id,
    );
    const packetFragmentCount = packetMatch?.fragments.length ?? 0;

    return {
      ...entry,
      packet_fragment_count: packetFragmentCount,
      coverage_complete:
        packetFragmentCount > 0 &&
        Number(entry.fragment_feedback_count) === packetFragmentCount,
    };
  });
}

function annotatePacketHistory(packetHistory, feedbackEvents, session) {
  const feedbackByPacketId = new Map(
    feedbackEvents.map((entry) => [entry.packet_id, entry]),
  );

  return packetHistory.map((packet, index) => {
    const previousPacket = packetHistory[index + 1] ?? null;
    const fragmentIds = new Set(
      (packet.fragments ?? []).map((fragment) => fragment.fragment_id),
    );
    const previousFragmentIds = new Set(
      (previousPacket?.fragments ?? []).map((fragment) => fragment.fragment_id),
    );
    let carryoverCount = 0;

    for (const fragmentId of fragmentIds) {
      if (previousFragmentIds.has(fragmentId)) {
        carryoverCount += 1;
      }
    }

    let droppedCount = 0;
    for (const fragmentId of previousFragmentIds) {
      if (!fragmentIds.has(fragmentId)) {
        droppedCount += 1;
      }
    }

    return {
      ...packet,
      feedback_event: feedbackByPacketId.get(packet.packet_id) ?? null,
      fragment_count: packet.fragments.length,
      new_count: Math.max(packet.fragments.length - carryoverCount, 0),
      carryover_count: carryoverCount,
      dropped_count: droppedCount,
      is_session_packet: packet.packet_id === session.packet_id,
    };
  });
}

function runDoltRows(sql) {
  return JSON.parse(runDolt(sql)).rows;
}

function loadAthenaState() {
  const session = readSession();
  const purposes = runDoltRows(
    `SELECT purpose_id, statement, status, success_criteria
     FROM purposes
     ORDER BY purpose_id DESC;`,
  );
  const allPacketRows = runDoltRows(
    `SELECT p.packet_id, p.purpose_id, pf.fragment_id, pf.kind, pf.position,
            IFNULL(pf.summary, pf.text) AS summary,
            IFNULL(pf.full_text, pf.text) AS full_text
     FROM packets p
     LEFT JOIN packet_fragments pf ON pf.packet_id = p.packet_id
     ORDER BY p.purpose_id DESC, p.packet_id DESC, pf.position ASC;`,
  );
  const allPacketHistory = groupPacketHistory(allPacketRows);
  const allFeedbackEvents = enrichFeedbackEvents(
    JSON.parse(
      runDolt(
        `SELECT fe.feedback_id, fe.purpose_id, fe.packet_id, fe.outcome,
                COUNT(ff.fragment_id) AS fragment_feedback_count
         FROM feedback_events fe
         LEFT JOIN feedback_fragments ff ON ff.feedback_id = fe.feedback_id
         GROUP BY fe.feedback_id, fe.purpose_id, fe.packet_id, fe.outcome
         ORDER BY fe.feedback_id DESC;`,
      ),
    ).rows,
    allPacketHistory,
  );
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
  const packetHistory = purposeId
    ? allPacketHistory.filter((entry) => entry.purpose_id === purposeId)
    : [];
  const packet =
    (packetIdFromSession
      ? packetHistory.find((entry) => entry.packet_id === packetIdFromSession)
      : null) ||
    packetHistory[0] ||
    null;
  const allFragments = runDoltRows(
    `SELECT fragment_id, kind, IFNULL(summary, text) AS summary, IFNULL(full_text, text) AS full_text
     FROM fragment_nodes
     ORDER BY fragment_id ASC;`,
  );

  let feedback = [];
  let feedbackEvent = null;
  let feedbackHistory = [];
  let feedbackEvents = [];

  if (purposeId) {
    feedbackEvents = allFeedbackEvents
      .filter((entry) => entry.purpose_id === purposeId)
      .slice(0, 12);

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

    feedbackHistory = JSON.parse(
      runDolt(
        `SELECT fe.feedback_id, fe.packet_id, fe.outcome, ff.fragment_id, ff.verdict,
                IFNULL(ff.reason, '') AS reason, ff.position
         FROM feedback_events fe
         JOIN feedback_fragments ff ON ff.feedback_id = fe.feedback_id
         WHERE fe.purpose_id = '${purposeId}'
         ORDER BY fe.feedback_id DESC, ff.position ASC;`,
      ),
    ).rows;
  }

  const purposeRequests = purposes.map((purposeEntry) => {
    const packets = annotatePacketHistory(
      allPacketHistory.filter(
        (packetEntry) => packetEntry.purpose_id === purposeEntry.purpose_id,
      ),
      allFeedbackEvents.filter(
        (event) => event.purpose_id === purposeEntry.purpose_id,
      ),
      session,
    );

    return {
      ...purposeEntry,
      packet_count: packets.length,
      latest_packet_id: packets[0]?.packet_id ?? null,
      oldest_packet_id: packets[packets.length - 1]?.packet_id ?? null,
      is_session_purpose: purposeEntry.purpose_id === session.purpose_id,
      packets,
    };
  });

  return {
    session,
    purpose,
    packet,
    packet_history: packetHistory,
    purpose_requests: purposeRequests,
    all_fragments: allFragments,
    feedback_event: feedbackEvent,
    feedback_events: feedbackEvents,
    feedback_fragments: feedback,
    recent_feedback: feedbackHistory,
    scores: summarizeScores(feedback),
    fragment_scores: computeFragmentScores(feedbackHistory),
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
    packet_history: [],
    purpose_requests: [],
    all_fragments: [],
    feedback_event: null,
    feedback_events: [],
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
