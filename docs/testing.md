# Testing

Benchmark and verification workflow for Athena memory work.

## Retrieval Benchmark

Purpose:
- measure retrieval-layer quality against frozen synthetic corpuses and tasks

Entrypoint:

```bash
scripts/athena-bench retrieval
```

Files:
- spec: `benchmarks/retrieval/benchmark_spec.json`
- corpuses: `benchmarks/retrieval/corpuses/*.json`
- tasks: `benchmarks/retrieval/tasks/*.json`
- runner: `src/benchmark/retrieval.rs`
- tests: `tests/retrieval_benchmark.rs`

Notes:
- benchmark wrapper initializes isolated per-run Athena Dolt state under `.athena/bench-dev/<subcommand>.*`
- benchmark wrapper clears per-run state after run unless `--keep-dev-db` is set
- when `--keep-dev-db` is set, wrapper prints kept path to stderr for inspection
- normal benchmark runs must not touch dogfood state in `.athena/db`
- benchmark calls retrieval code directly
- benchmark does not exercise full persisted Athena loop
- use it for ranking, trigger, supersession, and corpus-quality regressions

## Synthetic Creation Benchmark

Purpose:
- score frozen-context fragment proposals without running full repo-task agent loop
- isolate creation-layer judgment from retrieval, tool use, and task-solving variance

Entrypoint:

```bash
scripts/athena-bench creation
```

Files:
- spec: `benchmarks/creation/benchmark_spec.json`
- cases: `benchmarks/creation/cases/*.json`
- proposal examples: `benchmarks/creation/proposals/*.json`
- runner: `src/benchmark/creation.rs`
- tests: `tests/creation_benchmark.rs`

Each case contains:
- `purpose`
- `packet_fragments`
- `fragment_feedback`
- `outcome_note`
- gold expectations:
  - `should_create`
  - `max_fragments`
  - `preferred_kind`
  - `required_concepts`
  - `forbidden_concepts`
  - `concept_aliases`

Each proposal file contains:
- `case_id`
- `proposed_fragments[]`
  - `kind`
  - `summary`
  - `full_text`

Current scorer checks:
- correct create vs no-create decision
- fragment count limit
- preferred kind match
- required concept recall
- forbidden phrase avoidance

See [trajectory-benchmark.md](./trajectory-benchmark.md) for stateful repo-task benchmark plan and tracer-bullet shape.

## Trajectory Benchmark

Purpose:
- measure whether Codex completes chained repo tasks better with Athena than without it
- preserve evolving repo state across steps
- verify each step with hidden oracle tests

Tracer bullet:
- repo: `pallets/jinja`
- area: `jinja2.utils.LRUCache`
- steps: zero-capacity write discard, then `peek`, then `pop`

Entrypoint:

```bash
scripts/athena-bench trajectory --athena-mode off --keep-workdir
scripts/athena-bench trajectory --athena-mode current --keep-workdir
```

Current semantic baseline:
- mode: real Jinja `off`
- scope: 3 steps (`zero_capacity`, `peek`, `pop`)
- result: `3/3` resolved
- kept artifact: `/Users/yus/Projects/athena-v2/.athena/bench-dev/trajectory.tm1WV0/trajectory-run-1776074378359147000-21244-0`
- rerun command: `scripts/run_jinja_trajectory_tracer_bullet.sh off`

Files:
- plan: `docs/trajectory-benchmark.md`
- spec: `benchmarks/trajectory/jinja_tracer_bullet.json`
- prompts and hidden patches: `benchmarks/trajectory/jinja/*`
- runner: `src/benchmark/trajectory.rs`
- codex step helper: `scripts/athena-trajectory-codex-step`
- real-run wrapper: `scripts/run_jinja_trajectory_tracer_bullet.sh`
- tests: `tests/trajectory_benchmark.rs`

Per-step telemetry:
- `usage`: token counts from Codex JSON events when available
- `tool_counts`: completed Codex event item types with counts
- `observed_read_files`: best-effort repo files mentioned in command execution events
- `observed_edit_files`: repo files from Codex `file_change` events
- `changed_files`: repo files from post-step `git diff --name-only`
- each telemetry item carries source enum so downstream analysis can distinguish event-log facts from git-diff facts

Scorer output includes:
- per-case `score`
- decision correctness
- required concept hits and misses
- forbidden concept hits
- aggregates by family and difficulty

## Optional Blinded Judge

Cheap sanity-check only. Not source of truth.

Use blinded mini subagent to read:
- case file
- proposal file

Ask it for:
- `case_id`
- `pass|fail`
- short reason

Use this to spot obvious scorer blind spots.
Do not replace deterministic benchmark with judge output.

## Adding Cases

When adding retrieval cases:
- keep corpuses and tasks synthetic
- add new failure mode as new task family or corpus instead of mutating unrelated cases
- prefer small fixture sets with explicit gold ranks or required matches

When adding creation cases:
- freeze post-task context
- prefer concept ids over prose in gold
- include at least one no-create case
- include at least one “too generic” or “wrong kind” failure case
- keep outcome notes concise and reusable

## Verification Commands

Targeted benchmark tests:

```bash
cargo test retrieval_benchmark -- --nocapture
cargo test creation_benchmark -- --nocapture
```

Manual benchmark state helpers:

```bash
scripts/athena-bench setup
scripts/athena-bench clear
```

Session-close Athena checks:

```bash
scripts/athena_check.sh
```
