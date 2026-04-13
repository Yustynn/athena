Athena local Dolt state

- `db/`: tracked Dolt repository for Athena persisted memory
- `fragments.json`: tracked base fragments Athena assembles into packets
- `session.json`: untracked wrapper-managed current purpose/packet ids
- `.dolt-home/`: untracked Dolt CLI home used for local command state and telemetry files
- `scripts/athena`: persisted wrapper backed by `.athena/db`
- `scripts/athena-dev`: stateless experimentation wrapper

Prime current Athena context:

```bash
scripts/athena prime
```

Notes:
- run `scripts/athena prime` after new session, compaction, or `scripts/athena clear-session`
- create `.athena/PRIME.md` to override default prime output
- run `scripts/athena prime --export` to print built-in output even when override exists
- default prime output is workflow context only; inspect state separately with `scripts/athena current` or `scripts/athena latest-state`
- repo-local Codex SessionStart hook now injects `scripts/athena prime` from `.codex/hooks.json` when `codex_hooks` is enabled

Recommended Codex loop:

```bash
scripts/athena ensure-purpose "..." "..."
```

```bash
scripts/athena feedback partial feedback.json
```

Stateless dev loop:

```bash
scripts/athena-dev packet "..." "..."
```

```bash
scripts/athena-dev check-orientation request.json
```

```bash
scripts/athena-dev apply-feedback request.json
```

Low-level fallback:

```bash
cargo run --quiet --bin athena -- purpose create \
  --statement "..." \
  --success-criteria "..."
```

```bash
cargo run --quiet --bin athena -- purpose update \
  --purpose-id purpose-... \
  --statement "..." \
  --success-criteria "..."
```

```bash
echo '{"fragment_feedback":[...],"new_fragments":[...]}' | cargo run --quiet --bin athena -- feedback apply \
  --purpose-id purpose-... \
  --packet-id packet-... \
  --outcome partial
```

Install Codex MCP integration:

```bash
bash scripts/install_codex_athena_mcp.sh
```

This prebuilds `athena-mcp` into dedicated target dir, then registers MCP server with same `CARGO_TARGET_DIR`.
Purpose: avoid startup hangs when normal repo Cargo work holds default target-dir lock.

Install stable + dev MCP servers:

```bash
bash scripts/install_codex_athena_mcp.sh --with-dev
```

Recommendation:
- use `athena` MCP server for normal Codex work
- use `athena-dev` MCP server only when experimenting with packet/orientation behavior

Inspect current Athena tables:

```bash
HOME="$PWD/.athena/.dolt-home" dolt sql -q "show tables" -r json
```

Inspect persisted purposes:

```bash
HOME="$PWD/.athena/.dolt-home" dolt sql -q "select purpose_id, statement, success_criteria, status from purposes order by purpose_id desc;" -r json
```

Benchmark quick reference:

```bash
scripts/athena-bench retrieval
scripts/athena-bench creation
```

Benchmark wrapper uses isolated dev Athena state under `.athena/bench-dev` and clears it after each run, so dogfood `.athena/db` stays untouched.

See `docs/testing.md` for benchmark fixture layout, proposal format, and verification commands.
