# Athena Workflow Context

> Context recovery: run `scripts/athena prime` after new session, compaction, or `scripts/athena clear-session`
> Goal: teach Athena workflow and args. Do not dump live state here.

## What Athena Is

Athena is repo memory workflow for purpose-first work.

Shape:

```text
purpose -> packet -> work -> feedback -> next packet
```

Use repo bash wrappers for normal repo work:
- `scripts/athena` for persisted repo work backed by `.athena/db`
- `scripts/athena-dev` only for stateless experiments

Do not use Athena MCP for normal repo work.

## Core Rules

- Start substantive work with `scripts/athena ensure-purpose "..." "..."`
- Treat Athena as purpose-first, not latest-state-first
- If scope or done condition changes materially, update purpose before continuing
- After verification or meaningful learning, apply feedback for packet used during work
- Prefer wrapper defaults over manual purpose or packet id copying
- If Athena guidance conflicts with repo reality or tests, trust repo reality first, then record corrective feedback

## Essential Commands

Prime and orientation:

```bash
scripts/athena prime
scripts/athena current
scripts/athena latest-state
scripts/athena use-latest
scripts/athena clear-session
```

Purpose lifecycle:

```bash
scripts/athena ensure-purpose "statement" "success criteria"
scripts/athena create "statement" "success criteria"
scripts/athena update "statement" "success criteria"
```

Long forms:

```bash
scripts/athena ensure-purpose --statement "..." --success-criteria "..."
scripts/athena create --statement "..." --success-criteria "..."
scripts/athena update [--purpose-id purpose-...] --statement "..." --success-criteria "..."
```

Feedback:

```bash
scripts/athena feedback partial feedback.json
scripts/athena feedback [--purpose-id purpose-...] [--packet-id packet-...] --outcome success|partial|failed [--input feedback.json]
```

## What Commands Mean

- `prime`: print workflow instructions only
- `current`: show wrapper session state from `.athena/session.json`
- `latest-state`: show latest persisted state without changing session
- `use-latest`: copy latest persisted purpose and packet ids into wrapper session
- `clear-session`: remove wrapper session ids
- `ensure-purpose`: create purpose if none active, reuse matching active purpose, or update active purpose when statement or success criteria changed
- `create`: always create new purpose
- `update`: update purpose statement or success criteria
- `feedback`: apply packet feedback and optionally persist new fragments

## Common Workflows

Starting work:

```bash
scripts/athena prime
scripts/athena ensure-purpose "Fix trajectory benchmark telemetry" "Land change, verify tests, record reusable lessons"
```

Changing scope:

```bash
scripts/athena update "Expand Athena prime instructions" "Prime teaches Athena usage and args without extra docs"
```

Completing work:

1. Verify repo reality first: tests, benchmark, diff, docs.
2. Write feedback input covering every fragment in packet.
3. Apply feedback.

```bash
scripts/athena feedback success feedback.json
```

## Feedback Input

`feedback.json` must include exhaustive `fragment_feedback` for packet fragments.
Optional `new_fragments` adds reusable lessons.

Minimal shape:

```json
{
  "fragment_feedback": [
    {
      "fragment_id": "fragment-...",
      "verdict": "helped",
      "reason": "kept me on correct path"
    }
  ],
  "new_fragments": [
    {
      "kind": "procedure",
      "summary": "Use hidden verifier after repo mutation.",
      "full_text": "Trajectory benchmark should mutate repo first, then copy worktree to verifier, apply hidden patch, and run verifier there."
    }
  ]
}
```

Current fragment verdicts:
- `helped`
- `neutral`
- `wrong`

Good `new_fragments` candidates:
- correction to bad assumption
- canonical source pointer
- execution order future agent should follow
- durable workflow constraint
- reusable benchmark or failure-mode lesson

Do not store raw stack traces or one-off command dumps verbatim.

## State and Storage

- persisted state: `.athena/db`
- base fragments: `.athena/fragments.json`
- wrapper session ids: `.athena/session.json`
- benchmark isolated state: `.athena/bench-dev`

Benchmark runs should not write normal dogfood state in `.athena/db`.

## Benchmarks

Entrypoints:

```bash
scripts/athena-bench retrieval
scripts/athena-bench creation
scripts/athena-bench trajectory --athena-mode current --keep-workdir
```

If benchmark helper or session-start hook wants Athena usage instructions, inject `scripts/athena prime` output first.
Prime does not replace `scripts/athena ensure-purpose "..." "..."` when substantive work begins.

## Docs

- `.athena/README.md`
- `docs/testing.md`
- `AGENTS.md`

## Low-Level Fallback

Use only for debugging wrapper issues:

```bash
cargo run --quiet --bin athena -- purpose create --statement "..." --success-criteria "..."
cargo run --quiet --bin athena -- purpose update --purpose-id purpose-... --statement "..." --success-criteria "..."
echo '{"fragment_feedback":[...],"new_fragments":[...]}' | cargo run --quiet --bin athena -- feedback apply --purpose-id purpose-... --packet-id packet-... --outcome partial
```
