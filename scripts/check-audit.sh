#!/usr/bin/env bash
set -euo pipefail

trap 'echo "RESULT: FAIL"' ERR

cargo generate-lockfile || true
cargo audit

trap - ERR
echo "RESULT: PASS"
