#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="${ATHENA_TRAJECTORY_REPO_DIR:?}"
step_id="${ATHENA_TRAJECTORY_STEP_ID:?}"

case "$step_id" in
  step1_zero_capacity)
    patch_path="$script_dir/patches/step1.diff"
    ;;
  step2_peek)
    patch_path="$script_dir/patches/step2.diff"
    ;;
  step3_pop)
    patch_path="$script_dir/patches/step3.diff"
    ;;
  *)
    printf 'unknown step id: %s\n' "$step_id" >&2
    exit 1
    ;;
esac

git -C "$repo_dir" apply --whitespace=nowarn "$patch_path"
