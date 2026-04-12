Athena local Dolt state

- `db/`: tracked Dolt repository for Athena persisted memory
- `.dolt-home/`: untracked Dolt CLI home used for local command state and telemetry files

Inspect current Athena tables:

```bash
HOME="$PWD/.athena/.dolt-home" dolt sql -q "show tables" -r json
```

Inspect persisted purposes:

```bash
HOME="$PWD/.athena/.dolt-home" dolt sql -q "select purpose_id, statement, success_criteria, status from purposes order by purpose_id desc;" -r json
```
