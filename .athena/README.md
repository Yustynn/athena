Athena local Dolt state

- `db/`: tracked Dolt repository for Athena persisted memory and dogfood runs
- `.dolt-home/`: untracked Dolt CLI home used for local command state and telemetry files

Default dogfood path:

```bash
cargo run --quiet --bin dogfood
```

Isolated temp repo override:

```bash
ATHENA_DOGFOOD_DB_DIR="$(mktemp -d "${TMPDIR:-/tmp}/athena-dogfood.XXXXXX")" cargo run --quiet --bin dogfood
```
