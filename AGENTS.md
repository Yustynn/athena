# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd prime` for full workflow context.

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work atomically
bd close <id>         # Complete work
bd dolt push          # Push beads data only if Beads Dolt remote is configured
```

## Codex Cloud Setup

Use this repo script in Codex cloud environment setup:

```bash
bash scripts/codex_cloud_setup.sh
```

## Athena Session Loop

Use repo bash wrappers. Do not use Athena MCP for normal Codex work.

Persisted wrapper:

```bash
scripts/athena latest-state
scripts/athena ensure-purpose "..." "..."
scripts/athena update "..." "..."
scripts/athena feedback partial feedback.json
```

Tracked defaults:
- purposes, packets, feedback persist in `.athena/db`
- base fragments load from `.athena/fragments.json`
- wrapper session ids persist in untracked `.athena/session.json`

Stateless dev wrapper:

```bash
scripts/athena-dev packet "..." "..."
scripts/athena-dev check-orientation request.json
scripts/athena-dev apply-feedback request.json
```

Default:
- use `scripts/athena` for persisted repo work
- use `scripts/athena-dev` only for experimental stateless packet/orientation calls

Usage rules:
- at start of substantive work, call `scripts/athena latest-state` first
- if no active purpose fits, call `scripts/athena ensure-purpose "..." "..."`
- if scope or done condition changes materially, update purpose before continuing
- after verification or learning, apply Athena feedback for packet used during work
- prefer wrapper defaults over manual purpose/packet id copying; wrappers reuse `.athena/session.json`
- only write `new_fragments` for durable reusable knowledge, not transient task chatter
- durable reusable knowledge includes implementation lessons, not only product doctrine
- add `new_fragments` when session produces reusable lessons about storage access, performance limits, caching, polling, state-model distinctions, observability, or failure modes
- heuristic: if answer to "what should future agent do differently?" is non-trivial and likely reusable, write fragment
- do not store raw stack traces, one-off command output, ports, or transient debugging chatter; do store stable conclusions extracted from them
- if Athena output conflicts with repo reality or tests, trust repo reality first, then write corrective feedback
- `scripts/athena-dev` is for experiments only. Do not write its outputs into canonical Athena memory unless tests pass or user explicitly approves promotion

Raw low-level commands stay available for debugging:

```bash
cargo run --quiet --bin athena -- purpose create --statement "..." --success-criteria "..."
cargo run --quiet --bin athena -- purpose update --purpose-id purpose-... --statement "..." --success-criteria "..."
echo '{"fragment_feedback":[...],"new_fragments":[...]}' | cargo run --quiet --bin athena -- feedback apply --purpose-id purpose-... --packet-id packet-... --outcome partial
```

Repo also exposes minimal Athena stdio adapter for purpose -> packet -> feedback loop:

```bash
echo '{"kind":"assemble_packet","prompt":"...","success_criteria":"..."}' | cargo run --quiet --bin athena-stdio
```

To evaluate orientation:

```bash
echo '{"kind":"check_orientation","purpose":{...},"packet":{...},"response":{...}}' | cargo run --quiet --bin athena-stdio
```

To apply exhaustive fragment feedback and get next packet:

```bash
echo '{"kind":"apply_feedback","purpose":{...},"packet":{...},"feedback":{...}}' | cargo run --quiet --bin athena-stdio
```

If you expect Athena guidance during repo work, run wrapper or adapter explicitly. Nothing auto-injects packet data into chat session.

## Non-Interactive Shell Commands

**ALWAYS use non-interactive flags** with file operations to avoid hanging on confirmation prompts.

Shell commands like `cp`, `mv`, and `rm` may be aliased to include `-i` (interactive) mode on some systems, causing the agent to hang indefinitely waiting for y/n input.

**Use these forms instead:**
```bash
# Force overwrite without prompting
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# For recursive operations
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r source dest
```

**Other commands that may prompt:**
- `scp` - use `-o BatchMode=yes` for non-interactive
- `ssh` - use `-o BatchMode=yes` to fail instead of prompting
- `apt-get` - use `-y` flag
- `brew` - use `HOMEBREW_NO_AUTO_UPDATE=1` env var

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files
- This repo does not have Beads Dolt remote configured. Do NOT run `bd dolt push` unless remote is explicitly added later. Normal `git push` is enough here because Beads state is stored in repo.

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Session close** - Provide context for next session and ask: "What durable lessons from this session would change future implementation choices?" Convert answer into Athena `new_fragments`

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->

## Athena Check Workflow

Run this before session close to verify core Athena persistence and feedback-loop behavior:

```bash
scripts/athena_check.sh
```

What it does:
- runs targeted tests for feedback scoring
- runs feedback-loop packet-change coverage
- runs Dolt persistence e2e coverage

Optional git hook setup (recommended):

```bash
git config core.hooksPath .githooks
```

This enables the repository pre-push hook at `.githooks/pre-push`, which runs `scripts/athena_check.sh`.
