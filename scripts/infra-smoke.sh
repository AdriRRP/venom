#!/usr/bin/env bash
set -euo pipefail

profile="${VENOM_INFRA_PROFILE:-full}"

if [[ -n "${VENOM_INFRA_COMPOSE_FILE:-}" ]]; then
  case "$profile" in
    db|full)
      export VENOM_TEST_POSTGRES_URL="${VENOM_TEST_POSTGRES_URL:-postgres://venom:venom@127.0.0.1:55432/venom}"
      cargo test -p venom-api postgres_backend -- --nocapture
      exit 0
      ;;
    messaging)
      echo "SKIP: no standalone infra smoke is wired for profile $profile"
      exit 0
      ;;
    *)
      echo "ERROR: unsupported infra smoke profile: $profile" >&2
      echo "RESULT: FAIL"
      exit 1
      ;;
  esac
fi

case "$profile" in
  db|messaging)
    echo "SKIP: no standalone infra smoke is wired for profile $profile"
    exit 0
    ;;
  full)
    cargo run -p venom-domain --example syft_grype_live --all-features
    ;;
  *)
    echo "ERROR: unsupported infra smoke profile: $profile" >&2
    echo "RESULT: FAIL"
    exit 1
    ;;
esac
