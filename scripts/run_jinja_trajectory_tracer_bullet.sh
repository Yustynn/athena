#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
spec_path="$repo_root/benchmarks/trajectory/jinja_tracer_bullet.json"

usage() {
  cat <<'EOF'
usage:
  scripts/run_jinja_trajectory_tracer_bullet.sh off
  scripts/run_jinja_trajectory_tracer_bullet.sh current
  scripts/run_jinja_trajectory_tracer_bullet.sh preseed
  scripts/run_jinja_trajectory_tracer_bullet.sh both

notes:
  runs real jinja tracer bullet benchmark
  keeps benchmark workdir for inspection
  requires network for clone and package install
  current mode runs codex with Athena workflow prompt
  preseed mode adds blind benchmark-local Athena fragments before step 1
EOF
}

mode="${1:-both}"

case "$mode" in
  off|current|preseed)
    "$repo_root/scripts/athena-bench" trajectory \
      --keep-dev-db \
      --spec "$spec_path" \
      --athena-mode "$mode" \
      --keep-workdir
    ;;
  both)
    "$repo_root/scripts/athena-bench" trajectory \
      --keep-dev-db \
      --spec "$spec_path" \
      --athena-mode off \
      --keep-workdir
    "$repo_root/scripts/athena-bench" trajectory \
      --keep-dev-db \
      --spec "$spec_path" \
      --athena-mode current \
      --keep-workdir
    ;;
  --help|-h|help)
    usage
    ;;
  *)
    printf 'error: unknown mode: %s\n' "$mode" >&2
    usage >&2
    exit 1
    ;;
esac
