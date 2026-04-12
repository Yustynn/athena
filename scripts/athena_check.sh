#!/usr/bin/env bash
set -euo pipefail

cargo test -q --test feedback_scoring
cargo test -q --test feedback_loop_e2e
cargo test -q --test tracer_dolt_e2e
cargo test -q --test athena_cli_e2e
cargo test -q --test athena_mcp_e2e
