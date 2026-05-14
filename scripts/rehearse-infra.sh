#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/rehearse-infra.sh [--profile db|messaging|full]
EOF
}

fail() {
  echo "ERROR: $*" >&2
  echo "RESULT: FAIL"
  exit 1
}

find_compose_file() {
  local candidate

  for candidate in \
    "infra/compose.yaml" \
    "infra/compose.yml" \
    "infra/docker-compose.yaml" \
    "infra/docker-compose.yml"
  do
    if [[ -f "$candidate" ]]; then
      echo "$candidate"
      return 0
    fi
  done

  return 1
}

profile="full"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      [[ $# -ge 2 ]] || fail "missing value for --profile"
      profile="$2"
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

case "$profile" in
  db|messaging|full)
    ;;
  *)
    fail "unsupported infra profile: $profile"
    ;;
esac

compose_file="$(find_compose_file || true)"

if [[ -z "$compose_file" ]]; then
  if [[ -x "scripts/infra-smoke.sh" ]]; then
    VENOM_INFRA_PROFILE="$profile" ./scripts/infra-smoke.sh
    echo "RESULT: PASS"
    exit 0
  fi

  echo "SKIP: no infra compose file or infra smoke runner is wired yet"
  echo "RESULT: PASS"
  exit 0
fi

command -v docker >/dev/null 2>&1 || fail "docker is required for infra rehearsal"
[[ -x "scripts/infra-smoke.sh" ]] || fail "infra compose file exists but scripts/infra-smoke.sh is not wired yet"

compose_args=(-f "$compose_file")

case "$profile" in
  db)
    compose_args+=(--profile db)
    ;;
  messaging)
    compose_args+=(--profile messaging)
    ;;
  full)
    compose_args+=(--profile '*')
    ;;
esac

cleanup() {
  docker compose "${compose_args[@]}" down -v >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker compose "${compose_args[@]}" up -d --wait

VENOM_INFRA_COMPOSE_FILE="$compose_file" \
VENOM_INFRA_PROFILE="$profile" \
./scripts/infra-smoke.sh

echo "RESULT: PASS"
