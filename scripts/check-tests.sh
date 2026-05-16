#!/usr/bin/env bash
set -euo pipefail

trap 'echo "RESULT: FAIL"' ERR

cargo test --workspace --all-targets --all-features

if [[ -f apps/web/package.json ]]; then
  ./scripts/check-web.sh --lane test
fi

trap - ERR
echo "RESULT: PASS"
