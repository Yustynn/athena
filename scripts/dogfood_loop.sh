#!/usr/bin/env bash
set -euo pipefail

cargo test -q feedback_scoring
cargo test -q feedback_loop_e2e
cargo run --quiet --bin dogfood
