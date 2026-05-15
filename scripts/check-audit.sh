#!/usr/bin/env bash
set -euo pipefail

trap 'echo "RESULT: FAIL"' ERR

IGNORED_ADVISORIES=(
  # sqlx's meta-crate keeps mysql-only optional dependencies in Cargo.lock even when VENOM
  # ships a postgres-only runtime path. This advisory is unreachable in the current runtime.
  "RUSTSEC-2023-0071"
)

cargo generate-lockfile || true

audit_args=()
for advisory in "${IGNORED_ADVISORIES[@]}"; do
  audit_args+=(--ignore "$advisory")
done

cargo audit "${audit_args[@]}"

trap - ERR
echo "RESULT: PASS"
