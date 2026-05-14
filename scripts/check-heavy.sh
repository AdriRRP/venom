#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-heavy.sh --wave WXX-<slug> [--lane resilience|recovery|backpressure|stress]
EOF
}

fail() {
  echo "ERROR: $*" >&2
  echo "RESULT: FAIL"
  exit 1
}

validate_wave() {
  [[ "$1" =~ ^W[0-9]{2}-[a-z0-9]+(-[a-z0-9]+)*$ ]] || fail "invalid wave id: $1"
}

validate_active_wave() {
  local wave="$1"
  local active

  active="$(tr -d '\r' < docs/waves/ACTIVE)"
  if [[ "$active" != "NONE" && "$active" != "$wave" ]]; then
    fail "active wave is $active, not $wave"
  fi
}

wave=""
lane="stress"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --wave)
      [[ $# -ge 2 ]] || fail "missing value for --wave"
      wave="$2"
      shift 2
      ;;
    --lane)
      [[ $# -ge 2 ]] || fail "missing value for --lane"
      lane="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

[[ -n "$wave" ]] || fail "--wave is required"
validate_wave "$wave"
validate_active_wave "$wave"

case "$lane" in
  resilience|recovery|backpressure|stress)
    echo "SKIP: no heavy checks are wired yet for lane $lane"
    ;;
  *)
    fail "unsupported heavy lane: $lane"
    ;;
esac

echo "RESULT: PASS"
