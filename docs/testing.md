# Testing

Benchmark and verification workflow for Athena memory work.

## Retrieval Benchmark

Purpose:
- measure retrieval-layer quality against frozen synthetic corpuses and tasks

Entrypoint:

```bash
cargo run --quiet --bin athena-bench retrieval
```

Files:
- spec: `benchmarks/retrieval/benchmark_spec.json`
- corpuses: `benchmarks/retrieval/corpuses/*.json`
- tasks: `benchmarks/retrieval/tasks/*.json`
- runner: `src/benchmark/retrieval.rs`
- tests: `tests/retrieval_benchmark.rs`

Notes:
- benchmark calls retrieval code directly
- benchmark does not exercise full persisted Athena loop
- use it for ranking, trigger, supersession, and corpus-quality regressions

## Synthetic Creation Benchmark

Purpose:
- score frozen-context fragment proposals without running full repo-task agent loop
- isolate creation-layer judgment from retrieval, tool use, and task-solving variance

Entrypoint:

```bash
cargo run --quiet --bin athena-bench creation --proposals benchmarks/creation/proposals/baseline.json
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

Session-close Athena checks:

```bash
scripts/athena_check.sh
```
