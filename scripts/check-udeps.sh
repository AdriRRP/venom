#!/usr/bin/env bash
set -euo pipefail

cargo generate-lockfile || true
version_output="$(cargo +nightly udeps --version 2>&1)"
echo "$version_output"

args=(
  --workspace
  --all-targets
  --all-features
)

if [[ -n "${UDEPS_EXCLUDE:-}" ]]; then
  IFS=',' read -r -a excludes <<<"${UDEPS_EXCLUDE}"
  for crate_name in "${excludes[@]}"; do
    crate_name="$(echo "${crate_name}" | xargs)"
    if [[ -n "${crate_name}" ]]; then
      args+=(--exclude-crate "${crate_name}")
    fi
  done
fi

set +e
output="$(cargo +nightly udeps "${args[@]}" 2>&1)"
status=$?
set -e

if [[ $status -ne 0 ]]; then
  echo "$output"
  if grep -q 'valid options are "1" or "2"' <<<"$output"; then
    echo "ERROR: local cargo-udeps is too old for workspace resolver 3; use CI or upgrade cargo-udeps" >&2
  fi
  echo "RESULT: FAIL"
  exit $status
fi

echo "$output"
echo "RESULT: PASS"
