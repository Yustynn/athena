# Packet Assembler Plan

This plan turns current design into implementation path.

Linked design doc:

- [Agent Memory Schemas](./agent-memory-schemas.md)

## Goal

Build minimal packet-assembler tool in Rust that:

1. creates and tracks `purpose`
2. stores immutable fragment nodes plus small change graph
3. assembles `purpose_packet` for current purpose
4. records `orientation_response`
5. records `feedback_event` with exhaustive per-fragment feedback
6. supports tracer-bullet flow end to end before optimization

## Guiding Rules

- tracer bullet first
- vertical slices over horizontal infrastructure
- every slice gets unit tests plus one end-to-end test
- purpose-centered runtime, not startup-centered runtime
- immutable fragment nodes; no silent in-place canon rewrite
- keep runtime deterministic where possible
- use external mini-reasoner only behind clear interface and fallback

## Architecture Shape

```text
user prompt
  -> purpose builder
  -> candidate fetch
  -> heuristic filter/rank
  -> packet assembly
  -> task agent orientation_response
  -> optional correction_packet
  -> task execution
  -> feedback_event
  -> fragment graph updates
```

## Core Runtime Objects

Use schema doc as source of truth.

Implementation focus:

- `purpose`
- `fragment_node`
- `fragment_edge`
- `purpose_packet`
- `orientation_response`
- `correction_packet`
- `feedback_event`

## Fragment Graph Model

Nodes are immutable.

Each new correction creates new node, then edge.

Minimal edge types for v1:

- `supersedes`
- `merges`
- `rewrites`

Possible later:

- `narrows`
- `contradicts`

Selection rule:

- choose only active frontier nodes
- ignore nodes with active replacing descendant through `supersedes`, `merges`, or `rewrites`

## Vertical Slices

## Slice 0: Repo Skeleton + Types

Goal:

- create Rust crate layout
- define core types and enums
- no real runtime logic yet

Deliverables:

- type definitions
- serde support
- fixture directory
- basic round-trip serialization tests

Tests:

- unit: serde round-trip for each major type
- unit: enum parsing/validation

## Slice 1: Tracer Bullet End To End

Goal:

- run one full happy path with in-memory or SQLite-backed stub data

Path:

1. input prompt
2. create `purpose`
3. load fixed fragments from fixture
4. heuristically select small packet
5. accept mocked `orientation_response`
6. accept mocked `feedback_event`
7. update fragment scores / graph candidates

Deliverables:

- executable CLI or test harness
- one deterministic end-to-end flow

Tests:

- e2e: prompt -> purpose -> packet -> orientation -> feedback
- unit: packet slot fill from fixed candidate set
- unit: exhaustive `fragment_feedback[]` invariant

## Slice 2: Dolt Persistence

Goal:

- persist all runtime objects

Deliverables:

- schema migrations
- repositories for purpose, packet, fragment node, edge, feedback

Tests:

- unit: CRUD per table
- unit: immutable node insertion + edge linking
- e2e: tracer bullet using Dolt instead of fixture-only memory

## Slice 3: Heuristic Candidate Fetch + Rank

Goal:

- first real packet assembly without external reasoner

Logic:

- fetch by scope
- fetch by trigger condition
- drop superseded/stale
- score by freshness, correctness, usefulness, trigger match
- fill packet slots deterministically

Deliverables:

- candidate fetcher
- heuristic ranker
- packet slot filler

Tests:

- unit: superseded nodes excluded
- unit: stale nodes excluded
- unit: deterministic ranking from mixed candidates
- unit: duplicate concept collapse
- e2e: packet changes when purpose/trigger changes

## Slice 4: Orientation + Correction Loop

Goal:

- compare `orientation_response` against packet
- emit tiny `correction_packet` when misaligned

Checks:

- missing success criteria
- missing constraints
- wrong best path
- unresolved important question hidden

Deliverables:

- orientation checker
- correction packet builder

Tests:

- unit: aligned response yields no correction
- unit: missing constraint yields correction
- unit: wrong plan vs stop rule yields correction
- e2e: same purpose with correction creates improved second packet/response loop

## Slice 5: Feedback Ingestion + Graph Update

Goal:

- make feedback actionable

Behavior:

- every packet fragment must receive `fragment_feedback`
- missing fragment drafts create candidates
- incorrect/redundant feedback changes selection scores immediately
- merge/rewrite/supersede candidates create immutable replacement nodes and edges or queue candidate actions

Deliverables:

- feedback ingestor
- score update logic
- graph maintenance primitives

Tests:

- unit: exhaustive fragment feedback invariant enforced
- unit: incorrect fragment penalized
- unit: redundant pair suppresses weaker fragment in next packet
- unit: rewrite creates new node + edge, old node remains immutable
- e2e: second run after feedback yields different packet

## Slice 6: External Mini-Reasoner Interface

Goal:

- add pluggable reasoner without coupling runtime to vendor

Interface:

- `HeuristicReasoner`
- `MiniMaxReasoner`
- optional later `GlmReasoner`

Rules:

- mini-reasoner only sees shortlist
- strict JSON input/output
- hard fallback to heuristic selector on timeout or bad output

Tests:

- unit: provider response validation
- unit: invalid provider output falls back
- e2e: reasoner-enabled assembly still deterministic under fixture response

## Slice 7: Hardening + Realistic Scenario Tests

Goal:

- move from tracer bullet to credible tool

Need:

- realistic fixtures from mined convos
- packet-size constraints
- purpose transition coverage
- fragment frontier selection coverage

Tests:

- e2e: purpose fork creates child purpose and new packet
- e2e: superseded fragment never selected
- e2e: stale benchmark result replaced by newer result
- e2e: packet with redundant fragments becomes smaller after feedback
- e2e: correction packet fires before long run / benchmark

## Testing Strategy

## Unit Tests

Heavy coverage on:

- type validation
- ranking
- slot filling
- graph transitions
- feedback invariants
- fallback behavior

## End-to-End Tests

Every slice should add or extend one e2e scenario.

Minimum persistent scenarios:

1. happy path packet assembly
2. misaligned orientation -> correction
3. feedback marks fragment redundant -> next packet shrinks
4. feedback marks fragment incorrect -> replacement node wins
5. purpose fork -> child purpose packet differs from parent

## Tracer Bullet Constraints

Tracer bullet is allowed to be crude, but must be real.

Must be true even in Slice 1:

- actual typed objects
- actual packet assembly path
- actual feedback ingestion path
- actual invariant checks

Not enough:

- mocked architecture with no object flow
- TODO-only scaffolding

## Suggested Rust Layout

```text
src/
  purpose/
    mod.rs
    types.rs
    builder.rs
  fragment/
    mod.rs
    types.rs
    graph.rs
    score.rs
  packet/
    mod.rs
    types.rs
    assemble.rs
    slots.rs
  orientation/
    mod.rs
    check.rs
  feedback/
    mod.rs
    types.rs
    ingest.rs
  reasoner/
    mod.rs
    heuristic.rs
    minimax.rs
    prompt.rs
  storage/
    mod.rs
    dolt.rs
  tests/
    fixtures/
```

## Immediate Next Build Order

1. define core Rust types
2. add in-memory tracer bullet test
3. add Dolt persistence
4. implement heuristic packet assembly
5. implement orientation/correction
6. implement feedback ingestion + graph update
7. add MiniMax provider behind trait

## Handover Prompt

Use this in new convo.

```text
We already finished design discussion. Build tracer bullet for packet assembler in Rust, then flesh out in rigorous vertical slices.

Read first:
- docs/agent-memory-schemas.md
- docs/packet-assembler-plan.md

Current decisions:
- purpose-centered runtime, not startup-centered runtime
- no required purpose category
- feedback tied to both purpose_id and packet_id
- every fragment sent in packet must receive required fragment_feedback
- fragment nodes are immutable
- fragment changes happen through small graph edges
- current minimal edge types: supersedes, merges, rewrites
- prefer deterministic runtime with external mini-reasoner behind interface and fallback

What to do now:
1. scaffold Rust project structure for core modules
2. implement Slice 0 and Slice 1 from plan
3. include unit tests and one true end-to-end tracer-bullet test
4. keep code small and typed; no overbuilding
5. after tracer bullet passes, continue slice by slice with tests before broadening

Important constraints:
- vertical slices, not broad framework buildout
- rigorous e2e testing and unit tests while fleshing out slices
- use immutable node model from start
- packet assembly may be heuristic first; do not block on external reasoner
- if schema doc and code pressure conflict, update docs intentionally rather than drifting silently

At each milestone:
- say which slice is complete
- list tests added
- state remaining gap to next slice
```
