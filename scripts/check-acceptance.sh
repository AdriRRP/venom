#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "ERROR: $*" >&2
  echo "RESULT: FAIL"
  exit 1
}

if ! cargo run -p venom-domain --example acceptance --all-features; then
  fail "acceptance runner failed"
fi

echo "RESULT: PASS"
