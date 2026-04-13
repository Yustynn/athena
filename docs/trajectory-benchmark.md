# Trajectory Benchmark

Purpose:
- measure whether Codex completes chained repo tasks better with Athena than without it
- keep oracle hidden from agent
- preserve evolving repo state across steps

Non-goals for v1:
- evaluate memory write quality
- support many languages or all SWE-bench parsers
- produce publishable benchmark claims

## Reuse Boundary

Steal from SWE-bench:
- hidden `test_patch` per step
- `FAIL_TO_PASS` and `PASS_TO_PASS` task oracle split
- resolved scoring rule: all fail-to-pass and pass-to-pass cases must pass

Do not reuse directly:
- full fresh-clone evaluation runner
- image build stack
- fixed single-issue execution model

Reason:
- trajectory benchmark must continue from mutated repo state after each step
- SWE-bench harness assumes fresh checkout at one `base_commit`

## Benchmark Shape

```text
repo snapshot
  -> step 1 prompt
  -> agent run on mutable worktree
  -> copy worktree to verifier
  -> apply hidden test patch
  -> run hidden verifier
  -> step 2 prompt on original mutated worktree
  -> ...
```

Inputs:
- repo source: local path or git clone url + fixed revision
- optional setup commands
- runner command
- step prompts
- hidden test patches
- verifier command
- `FAIL_TO_PASS`
- `PASS_TO_PASS`

Outputs:
- per-step resolved / unresolved
- fail-to-pass rate
- pass-to-pass rate
- runner and verifier wall time
- usage tokens from Codex JSON events when available
- observed tool counts, read files, edit files, and git-changed files with source enums
- failure description from runner or verifier output when step exits nonzero
- aggregate resolved count and resolution rate

## Tracer Bullet

Tracer bullet scope:
- one real repo: `pallets/jinja`
- one file area: `jinja2.utils.LRUCache`
- three steps
- one parser family: `pytest`
- one manual runner path: `codex exec`
- one offline fixture test path for CI

Jinja tracer bullet tasks:
1. zero-capacity cache should discard writes without crashing
2. add `peek(key, default=None)` that does not update recency
3. add `pop(key, default=missing)` that removes entry without disturbing remaining recency

## Current Semantic Baseline

- mode: real Jinja `off`
- scope: all 3 tracer-bullet steps
- result: `3/3` resolved
- kept artifact: `/Users/yus/Projects/athena-v2/.athena/bench-dev/trajectory.tm1WV0/trajectory-run-1776074378359147000-21244-0`
- rerun command: `scripts/run_jinja_trajectory_tracer_bullet.sh off`

Why this slice:
- small enough to author now
- same class across both steps, so later step can benefit from earlier discovery
- hidden tests are deterministic
- no need to understand whole template engine

## Current Layout

```text
benchmarks/trajectory/
  jinja_tracer_bullet.json
  jinja/
    blind_fragments.json
    step1.prompt.md
    step2.prompt.md
    step3.prompt.md
    step1.hidden.diff
    step2.hidden.diff
    step3.hidden.diff
```

Runner contract:
- benchmark runner sets env vars with repo path, prompt path, message file, and Athena mode
- in `current` mode, benchmark runner writes repo-local `.codex/hooks.json` and session-start hook into cloned repo before runner starts
- generated hook emits `scripts/athena prime` output from athena-v2 host repo as SessionStart additional context
- in `preseed` mode, benchmark runner also seeds benchmark-local Athena Dolt storage before step 1 from blind fragment fixture declared in spec
- preseed source metadata must point at clone-repo files, not prompts, hidden diffs, run logs, or athena-v2 docs
- external runner command mutates repo in place
- Codex helper script lives at `scripts/athena-trajectory-codex-step`
- helper runs `codex exec --json` so runner stdout is raw JSONL event log
- helper still appends explicit Athena `ensure-purpose` guidance in Athena-enabled modes

Telemetry sources:
- `codex_event_log`: parsed from Codex JSONL events on runner stdout
- `git_diff`: parsed from post-step `git diff --name-only`
- `runner_stdout` / `runner_stderr`: fallback source for runner failures
- `verifier_stdout` / `verifier_stderr`: fallback source for verifier failures

Important env vars:
- `ATHENA_TRAJECTORY_REPO_DIR`
- `ATHENA_TRAJECTORY_STEP_ID`
- `ATHENA_TRAJECTORY_STEP_PROMPT_FILE`
- `ATHENA_TRAJECTORY_MESSAGE_FILE`
- `ATHENA_TRAJECTORY_ATHENA_MODE`
- `ATHENA_DB_PATH`
- `ATHENA_DOLT_HOME` in `preseed` mode

## First Implementation Slice

1. generic trajectory runner in Rust
2. local synthetic fixture for deterministic tests
3. real Jinja spec for manual execution
4. Codex step helper script
5. docs and wrapper command in `scripts/athena-bench`

## Risks

- real Jinja run needs network for clone and package install
- Codex CLI benchmark runs may need unsandboxed access to Codex session files
- generic `pytest` parser is enough for tracer bullet, not for broad SWE-bench reuse
- one early failed step can poison later steps by design

## Next After Tracer Bullet

- add token accounting from Codex JSON events if stable enough
- vendor more SWE-bench parsers instead of generic `pytest` parsing only
- add more repos and difficulty tiers
