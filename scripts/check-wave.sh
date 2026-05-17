#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-wave.sh --wave WXX-<slug> [--lane unit|integration|infra|acceptance|e2e|contract|full]
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

infra_profile_from_wave() {
  local wave="$1"
  local doc="docs/waves/${wave}.md"
  local profile

  [[ -f "$doc" ]] || fail "missing wave doc: $doc"

  profile="$(sed -n 's/^Infra profile: `\(.*\)`$/\1/p' "$doc" | head -n 1)"
  [[ -n "$profile" ]] || profile="none"
  echo "$profile"
}

has_acceptance_specs() {
  find features -type f -name '*.feature' ! -path 'features/e2e/*' ! -name 'FEATURE-TEMPLATE.feature' | grep -q .
}

has_e2e_specs() {
  if find features/e2e -type f ! -name '.gitkeep' 2>/dev/null | grep -q .; then
    return 0
  fi

  find apps/web/e2e -type f ! -name '.gitkeep' 2>/dev/null | grep -q .
}

has_contract_specs() {
  find tests/contracts -type f ! -name 'README.md' | grep -q .
}

run_acceptance_lane() {
  if has_acceptance_specs; then
    ./scripts/check-acceptance.sh
    return 0
  fi

  echo "SKIP: no acceptance specs"
}

run_e2e_lane() {
  if find features/e2e -type f ! -name '.gitkeep' 2>/dev/null | grep -q .; then
    fail "feature e2e specs exist but no feature e2e runner is wired yet"
  fi

  if find apps/web/e2e -type f ! -name '.gitkeep' 2>/dev/null | grep -q .; then
    ./scripts/check-web.sh --lane e2e
    return 0
  fi

  echo "SKIP: no e2e specs"
}

run_contract_lane() {
  if has_contract_specs; then
    ./scripts/check-contracts.sh
    return 0
  fi

  echo "SKIP: no contract checks"
}

repo_has_web_app() {
  [[ -f apps/web/package.json ]]
}

run_infra_lane() {
  local profile="$1"

  if [[ "$profile" == "none" ]]; then
    echo "SKIP: wave infra profile is none"
    return 0
  fi

  ./scripts/rehearse-infra.sh --profile "$profile"
}

run_full_wave() {
  local profile="$1"

  ./scripts/check-git-discipline.sh --mode wave --wave "$wave"
  ./scripts/check-quality.sh
  ./scripts/check-tests.sh
  if repo_has_web_app; then
    ./scripts/check-web.sh --lane build
  fi
  run_infra_lane "$profile"
  run_acceptance_lane
  run_e2e_lane
  run_contract_lane
}

wave=""
lane="full"

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
infra_profile="$(infra_profile_from_wave "$wave")"

case "$lane" in
  unit|integration)
    ./scripts/check-tests.sh
    if repo_has_web_app; then
      ./scripts/check-web.sh --lane build
    fi
    ;;
  infra)
    run_infra_lane "$infra_profile"
    ;;
  acceptance)
    run_acceptance_lane
    ;;
  e2e)
    run_e2e_lane
    ;;
  contract)
    run_contract_lane
    ;;
  full)
    run_full_wave "$infra_profile"
    ;;
  *)
    fail "unsupported wave lane: $lane"
    ;;
esac

echo "RESULT: PASS"
