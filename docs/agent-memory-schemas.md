# Agent Memory Schemas

This document defines current schema set for memory-aware ephemeral agents.

Core principle:

- `purpose` is canonical work unit
- `purpose_id` ties runtime actions and feedback to exact goal
- `packet_id` ties feedback to exact packet version
- no required purpose category

## Scope

These are schemas flowing to/from agent, plus minimal supporting schema needed to assemble packets.

Included:

1. `purpose`
2. `fragment`
3. `purpose_packet`
4. `orientation_response`
5. `correction_packet`
6. `feedback_event`

Not included:

- storage/index internals
- retrieval/ranking algorithm
- merge/review workflow

## 1. Purpose

Canonical work unit. Purpose is not same as startup. New purpose can appear at startup or any later task transition.

```text
purpose
  purpose_id
  parent_purpose_id?
  status
  statement
  success_criteria
  constraints[]
  invariants[]
  scope
  created_at
  updated_at
```

### Fields

- `purpose_id`: stable identity for this goal
- `parent_purpose_id`: optional link to parent purpose when goal forks or narrows
- `status`: `active | completed | blocked | abandoned | superseded`
- `statement`: one-sentence purpose in plain language
- `success_criteria`: concrete done/better condition
- `constraints[]`: hard limits or required conditions
- `invariants[]`: things that must remain true even if plan changes
- `scope`: boundary of this purpose
- `created_at`: creation time
- `updated_at`: last purpose revision time

### Notes

- no required purpose category
- if helper labels are needed later, derive them from purpose and packet state

## 2. Fragment

Atomic memory unit used by packet assembler.

```text
fragment
  fragment_id
  type
  text
  scope
  trigger_conditions[]
  state
  concept_key
  usefulness_score
  correctness_confidence
  durability_score
  stale_after?
  supports[]
  supersedes[]
  contradicts[]
  provenance[]
```

### Fields

- `fragment_id`: stable fragment identity
- `type`: `doctrine | procedure | pitfall | preference | context`
- `text`: atomic claim or rule
- `scope`: where fragment applies
- `trigger_conditions[]`: when packet assembler should consider it
- `state`: `scratch | durable | deferred | stale | superseded`
- `concept_key`: dedupe/merge key for near-duplicates
- `usefulness_score`: current usefulness estimate
- `correctness_confidence`: confidence fragment is right
- `durability_score`: estimate of reuse across future purposes
- `stale_after`: optional expiry signal
- `supports[]`: related fragment ids
- `supersedes[]`: older fragment ids displaced by this one
- `contradicts[]`: fragment ids in conflict
- `provenance[]`: evidence links, convo refs, or source pointers

## 3. Purpose Packet

Runtime packet assembled for one purpose instance.

```text
purpose_packet
  packet_id
  purpose_id
  purpose_statement
  success_criteria
  current_state
  current_best_path?
  current_fork?
  stop_rule?
  open_questions[]
  fragments[]
```

### Fields

- `packet_id`: identity for this packet version
- `purpose_id`: purpose served by this packet
- `purpose_statement`: copy of current purpose statement
- `success_criteria`: visible runtime success condition
- `current_state`: short live snapshot
- `current_best_path`: recommended next move right now
- `current_fork`: active branch or decision fork, if any
- `stop_rule`: explicit stop/cutoff rule, if any
- `open_questions[]`: known unknowns that matter now
- `fragments[]`: selected fragment payloads or fragment refs

### Notes

- this replaces idea of special `startup packet`
- startup is only first purpose packet

## 4. Orientation Response

Agent restates understanding before action.

```text
orientation_response
  purpose_id
  packet_id
  agent_goal_statement
  agent_success_criteria
  agent_constraints[]
  agent_plan
  agent_uncertainties[]
  alignment
```

### Fields

- `purpose_id`: purpose being answered
- `packet_id`: packet agent saw
- `agent_goal_statement`: agent restatement of goal
- `agent_success_criteria`: agent restatement of done/better condition
- `agent_constraints[]`: constraints agent thinks matter
- `agent_plan`: immediate plan
- `agent_uncertainties[]`: what remains unclear
- `alignment`: `aligned | partial | misaligned`

### Notes

- this is where `first, tell me what you think` lives
- system compares this response against packet and decides whether correction is needed

## 5. Correction Packet

Small follow-up packet when orientation is incomplete or misaligned.

```text
correction_packet
  correction_id
  purpose_id
  packet_id
  reason
  missing_constraints[]
  missing_invariants[]
  missing_questions[]
  added_fragments[]
```

### Fields

- `correction_id`: identity for this correction event
- `purpose_id`: target purpose
- `packet_id`: packet version being corrected
- `reason`: why correction fired
- `missing_constraints[]`: constraints agent omitted
- `missing_invariants[]`: invariants agent omitted
- `missing_questions[]`: questions that should be resolved before action
- `added_fragments[]`: small set of fragments injected now

### Notes

- correction packet should stay tiny
- goal is not to resend full packet

## 6. Feedback Event

Feedback tied to exact purpose and packet version.

```text
feedback_event
  feedback_id
  purpose_id
  packet_id
  phase
  outcome
  purpose_fit
  packet_fit
  fragment_feedback[]
  missing_fragments[]
  notes?
  created_at
```

### Fields

- `feedback_id`: identity for this feedback record
- `purpose_id`: purpose being judged
- `packet_id`: packet version being judged
- `phase`: `startup | after_orientation | before_tool_use | before_edit | before_benchmark | before_long_run | post_task`
- `outcome`: `success | partial | failed | abandoned | superseded`
- `purpose_fit`: `good | partial | bad`
- `packet_fit`: packet-quality object
- `fragment_feedback[]`: required exhaustive feedback for every fragment sent in packet
- `missing_fragments[]`: fragment candidates that should have existed
- `notes`: optional freeform note
- `created_at`: feedback time

### Packet Fit Object

```text
packet_fit
  purpose_alignment
  size
  specificity
  timing
```

- `purpose_alignment`: `good | partial | bad`
- `size`: `good | too_long | too_short`
- `specificity`: `good | too_vague | too_narrow`
- `timing`: `good | too_late | too_early`

### Fragment Feedback Shapes

`fragment_feedback[]` is required and exhaustive.

Invariant:

```text
set(fragment_feedback.fragment_id)
==
set(purpose_packet.fragments.fragment_id)
```

Minimal supporting item shapes:

```text
fragment_feedback
  fragment_id
  verdict
  usefulness
  correctness
  redundancy
  timing
  should_reuse
  reason?
  redundant_with?

missing_fragment
  proposed_type
  proposed_text
  scope
  trigger_condition
  when_needed
  why_it_mattered
```

### Fragment Feedback Fields

- `fragment_id`: fragment being judged
- `verdict`: `helped | neutral | wrong | redundant | late`
- `usefulness`: `high | medium | low | none`
- `correctness`: `confirmed | uncertain | incorrect`
- `redundancy`: `unique | overlap | duplicate`
- `timing`: `good | too_early | too_late`
- `should_reuse`: `yes | maybe | no`
- `reason`: optional short explanation
- `redundant_with`: optional fragment id that covered this fragment better

## Direction Of Flow

### Server -> Agent

- `purpose_packet`
- `correction_packet`

### Agent -> Server

- `orientation_response`
- `feedback_event`

### Shared Runtime Identity

- `purpose`
- `fragment`

## Minimal Runtime Loop

```text
purpose created
  -> purpose_packet assembled
  -> agent returns orientation_response
  -> server may send correction_packet
  -> agent acts
  -> agent/server emits feedback_event
```

## Current Decision Summary

- no required purpose category
- purpose can begin at startup or any later transition
- feedback must tie to both `purpose_id` and `packet_id`
- packet quality matters, not only fragment quality
- redundancy feedback is first-class
