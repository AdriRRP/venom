#!/usr/bin/env bash
set -euo pipefail

trap 'echo "RESULT: FAIL"' ERR

cargo run -p venom-domain --example contracts --all-features

trap - ERR
echo "RESULT: PASS"
