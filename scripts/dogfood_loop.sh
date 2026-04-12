#!/usr/bin/env bash
set -euo pipefail

DOGFOOD_DB_DIR="$(mktemp -d "${TMPDIR:-/tmp}/athena-dogfood.XXXXXX")"
trap 'rm -rf "$DOGFOOD_DB_DIR"' EXIT

cargo test -q --test feedback_scoring
cargo test -q --test feedback_loop_e2e

DOGFOOD_OUTPUT="$(ATHENA_DOGFOOD_DB_DIR="$DOGFOOD_DB_DIR" cargo run --quiet --bin dogfood)"
printf '%s\n' "$DOGFOOD_OUTPUT"

FIRST_LINE="$(printf '%s\n' "$DOGFOOD_OUTPUT" | rg -m1 '^first packet fragments:' || true)"
SECOND_LINE="$(printf '%s\n' "$DOGFOOD_OUTPUT" | rg -m1 '^second packet fragments:' || true)"

if [[ -z "$FIRST_LINE" || -z "$SECOND_LINE" ]]; then
  echo "dogfood output missing fragment summary lines" >&2
  exit 1
fi

FIRST_IDS="${FIRST_LINE#first packet fragments: }"
SECOND_IDS="${SECOND_LINE#second packet fragments: }"

if [[ "$FIRST_IDS" == "$SECOND_IDS" ]]; then
  echo "feedback loop did not change packet fragments" >&2
  exit 1
fi

echo "feedback loop changed packet fragments as expected"
