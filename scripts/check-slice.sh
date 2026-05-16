#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-slice.sh --wave WXX-<slug> --slice WXX-SYY [--lane unit|integration|infra|acceptance|e2e|contract] [--path <repo-path>...]
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

validate_slice() {
  [[ "$1" =~ ^W[0-9]{2}-S[0-9]{2}$ ]] || fail "invalid slice id: $1"
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
  find features/e2e -type f ! -name '.gitkeep' | grep -q .
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
  if has_e2e_specs; then
    fail "e2e specs exist but no e2e runner is wired yet"
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

paths_touch_web() {
  for path in "${paths[@]}"; do
    if [[ "$path" == apps/web* ]]; then
      return 0
    fi
  done
  return 1
}

run_infra_lane() {
  local profile="$1"

  if [[ "$profile" == "none" ]]; then
    echo "SKIP: wave infra profile is none"
    return 0
  fi

  ./scripts/rehearse-infra.sh --profile "$profile"
}

wave=""
slice=""
declare -a lanes=()
declare -a paths=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --wave)
      [[ $# -ge 2 ]] || fail "missing value for --wave"
      wave="$2"
      shift 2
      ;;
    --slice)
      [[ $# -ge 2 ]] || fail "missing value for --slice"
      slice="$2"
      shift 2
      ;;
    --lane)
      [[ $# -ge 2 ]] || fail "missing value for --lane"
      lanes+=("$2")
      shift 2
      ;;
    --path)
      [[ $# -ge 2 ]] || fail "missing value for --path"
      paths+=("$2")
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
[[ -n "$slice" ]] || fail "--slice is required"

validate_wave "$wave"
validate_slice "$slice"
[[ "$slice" == "${wave%%-*}"-S* ]] || fail "slice $slice does not belong to wave $wave"
validate_active_wave "$wave"
infra_profile="$(infra_profile_from_wave "$wave")"

if [[ ${#paths[@]} -gt 0 ]]; then
  printf 'PATHS: %s\n' "${paths[*]}"
fi

./scripts/check-quality.sh

if [[ ${#lanes[@]} -eq 0 ]]; then
  ./scripts/check-tests.sh
  if paths_touch_web; then
    ./scripts/check-web.sh --lane build
  fi
  echo "RESULT: PASS"
  exit 0
fi

for lane in "${lanes[@]}"; do
  case "$lane" in
    unit|integration)
      ./scripts/check-tests.sh
      if paths_touch_web; then
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
    *)
      fail "unsupported slice lane: $lane"
      ;;
  esac
done

echo "RESULT: PASS"
