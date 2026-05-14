#!/usr/bin/env bash
set -euo pipefail

trap 'echo "RESULT: FAIL"' ERR

cargo test --workspace --all-targets --all-features

trap - ERR
echo "RESULT: PASS"
