#!/usr/bin/env bash
set -euo pipefail

install_dev=0
if [[ "${1:-}" == "--with-dev" ]]; then
  install_dev=1
fi

repo_root="$(cd "$(dirname "$0")/.." && pwd)"

codex mcp remove athena >/dev/null 2>&1 || true
codex mcp add athena -- cargo run --quiet --manifest-path "$repo_root/Cargo.toml" --bin athena-mcp -- stable

if [[ "$install_dev" -eq 1 ]]; then
  codex mcp remove athena-dev >/dev/null 2>&1 || true
  codex mcp add athena-dev -- cargo run --quiet --manifest-path "$repo_root/Cargo.toml" --bin athena-mcp -- dev
fi
