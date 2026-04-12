#!/usr/bin/env bash
set -euo pipefail

local_bin="$HOME/.local/bin"
path_line='export PATH="$HOME/.local/bin:$PATH"'

mkdir -p "$local_bin"
export PATH="$local_bin:$PATH"

# Codex setup runs in separate shell from agent phase. Persist PATH for agent.
if [ ! -f "$HOME/.bashrc" ] || ! grep -Fqx "$path_line" "$HOME/.bashrc"; then
  printf '\n%s\n' "$path_line" >> "$HOME/.bashrc"
fi

if ! command -v bd >/dev/null 2>&1; then
  curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash
fi

bd version

if command -v codex >/dev/null 2>&1; then
  if [[ "${ATHENA_INSTALL_DEV_MCP:-0}" == "1" ]]; then
    bash scripts/install_codex_athena_mcp.sh --with-dev
  else
    bash scripts/install_codex_athena_mcp.sh
  fi
fi
