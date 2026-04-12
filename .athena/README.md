Athena local Dolt state

- `db/`: tracked Dolt repository for Athena persisted memory
- `fragments.json`: tracked base fragments Athena assembles into packets
- `.dolt-home/`: untracked Dolt CLI home used for local command state and telemetry files
- stable Codex MCP server: persisted tools backed by `.athena/db`
- dev Codex MCP server: stateless experimentation tools

Start Athena loop:

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
