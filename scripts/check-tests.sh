#!/usr/bin/env bash
set -euo pipefail

trap 'echo "RESULT: FAIL"' ERR

if [[ "${VENOM_REQUIRE_POSTGRES_TESTS:-0}" == "1" && -z "${VENOM_TEST_POSTGRES_URL:-}" ]]; then
  echo "ERROR: VENOM_TEST_POSTGRES_URL is required when PostgreSQL tests are mandatory" >&2
  false
fi

cargo test --workspace --all-targets --all-features

if [[ -f apps/web/package.json ]]; then
  ./scripts/check-web.sh --lane test
fi

trap - ERR
echo "RESULT: PASS"
