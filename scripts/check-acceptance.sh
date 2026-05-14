#!/usr/bin/env bash
set -euo pipefail

trap 'echo "RESULT: FAIL"' ERR

cargo run -p venom-domain --example acceptance --all-features

trap - ERR
echo "RESULT: PASS"
