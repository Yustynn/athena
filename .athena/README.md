Athena local Dolt state

- `db/`: tracked Dolt repository for Athena persisted memory
- `fragments.json`: tracked base fragments Athena assembles into packets
- `.dolt-home/`: untracked Dolt CLI home used for local command state and telemetry files

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

Inspect current Athena tables:

```bash
HOME="$PWD/.athena/.dolt-home" dolt sql -q "show tables" -r json
```

Inspect persisted purposes:

```bash
HOME="$PWD/.athena/.dolt-home" dolt sql -q "select purpose_id, statement, success_criteria, status from purposes order by purpose_id desc;" -r json
```
