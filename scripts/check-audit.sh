#!/usr/bin/env bash
set -euo pipefail

trap 'echo "RESULT: FAIL"' ERR

IGNORED_ADVISORIES=(
  # sqlx's meta-crate keeps mysql-only optional dependencies in Cargo.lock even when VENOM
  # ships a postgres-only runtime path. This advisory is unreachable in the current runtime.
  "RUSTSEC-2023-0071"
)

[[ -f Cargo.lock ]] || {
  echo "ERROR: missing committed Cargo.lock" >&2
  false
}

audit_args=()
for advisory in "${IGNORED_ADVISORIES[@]}"; do
  audit_args+=(--ignore "$advisory")
done

cargo audit --file Cargo.lock "${audit_args[@]}"

if [[ -f apps/web/package.json ]]; then
  [[ -f apps/web/package-lock.json ]] || {
    echo "ERROR: missing committed apps/web/package-lock.json" >&2
    false
  }
  npm --prefix apps/web audit --package-lock-only --audit-level=high
fi

trap - ERR
echo "RESULT: PASS"
