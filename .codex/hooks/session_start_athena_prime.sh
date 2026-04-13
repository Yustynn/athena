#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
prime_output="$("$repo_root/scripts/athena" prime)"
escaped_output="$(printf '%s' "$prime_output" | python3 -c 'import json, sys; print(json.dumps(sys.stdin.read()))')"

printf '{'
printf '"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":%s}' "$escaped_output"
printf '}\n'
