#!/usr/bin/env bash
set -euo pipefail

install_dev=0
if [[ "${1:-}" == "--with-dev" ]]; then
  install_dev=1
fi

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
mcp_target_dir="$repo_root/target/athena-mcp-mcp"

CARGO_TARGET_DIR="$mcp_target_dir" cargo build --quiet --manifest-path "$repo_root/Cargo.toml" --bin athena-mcp

codex mcp remove athena >/dev/null 2>&1 || true
codex mcp add athena \
  --env "CARGO_TARGET_DIR=$mcp_target_dir" \
  -- cargo run --quiet --manifest-path "$repo_root/Cargo.toml" --bin athena-mcp -- stable

if [[ "$install_dev" -eq 1 ]]; then
  codex mcp remove athena-dev >/dev/null 2>&1 || true
  codex mcp add athena-dev \
    --env "CARGO_TARGET_DIR=$mcp_target_dir" \
    -- cargo run --quiet --manifest-path "$repo_root/Cargo.toml" --bin athena-mcp -- dev
fi
