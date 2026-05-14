#!/usr/bin/env bash
set -euo pipefail

trap 'echo "RESULT: FAIL"' ERR

cargo fmt --all -- --check

cargo clippy \
  --workspace \
  --all-targets \
  --all-features \
  -- \
  -D warnings \
  -W clippy::all \
  -W clippy::pedantic \
  -W clippy::nursery \
  -W clippy::perf \
  -W clippy::cargo \
  -A clippy::multiple_crate_versions

trap - ERR
echo "RESULT: PASS"
