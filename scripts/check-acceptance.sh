#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "ERROR: $*" >&2
  echo "RESULT: FAIL"
  exit 1
}

log_file="$(mktemp "${TMPDIR:-/tmp}/venom-acceptance.XXXXXX")"
cleanup() {
  rm -f "${log_file}"
}
trap cleanup EXIT

if ! cargo run -p venom-domain --example acceptance --all-features | tee "${log_file}"; then
  fail "acceptance runner failed"
fi

if grep -Eq 'Step skipped:|scenario[s]? \([0-9]+ skipped|steps \([^)]*skipped' "${log_file}"; then
  fail "acceptance contains skipped observable coverage"
fi

echo "RESULT: PASS"
