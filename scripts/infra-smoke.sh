#!/usr/bin/env bash
set -euo pipefail

profile="${VENOM_INFRA_PROFILE:-full}"

case "$profile" in
  db|messaging)
    echo "SKIP: no standalone infra smoke is wired for profile $profile"
    exit 0
    ;;
  full)
    ;;
  *)
    echo "ERROR: unsupported infra smoke profile: $profile" >&2
    echo "RESULT: FAIL"
    exit 1
    ;;
esac

cargo run -p venom-domain --example syft_grype_live --all-features
