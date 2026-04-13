#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="${ATHENA_TRAJECTORY_REPO_DIR:?}"
step_id="${ATHENA_TRAJECTORY_STEP_ID:?}"

case "$step_id" in
  step1_zero_capacity)
    patch_path="$script_dir/patches/step1.diff"
    input_tokens=101
    cached_input_tokens=41
    output_tokens=11
    ;;
  step2_peek)
    patch_path="$script_dir/patches/step2.diff"
    input_tokens=102
    cached_input_tokens=42
    output_tokens=12
    ;;
  step3_pop)
    patch_path="$script_dir/patches/step3.diff"
    input_tokens=103
    cached_input_tokens=43
    output_tokens=13
    ;;
  *)
    printf 'unknown step id: %s\n' "$step_id" >&2
    exit 1
    ;;
esac

printf '%s\n' '{"type":"turn.started"}'
printf '%s\n' '{"type":"item.completed","item":{"id":"item_read","type":"command_execution","command":"cat cachelib.py tests/test_public.py","aggregated_output":"","exit_code":0,"status":"completed"}}'
printf '{"type":"item.completed","item":{"id":"item_edit","type":"file_change","changes":[{"path":"%s/cachelib.py","kind":"update"}],"status":"completed"}}\n' "$repo_dir"
printf '{"type":"turn.completed","usage":{"input_tokens":%s,"cached_input_tokens":%s,"output_tokens":%s}}\n' \
  "$input_tokens" "$cached_input_tokens" "$output_tokens"

git -C "$repo_dir" apply --whitespace=nowarn "$patch_path"
