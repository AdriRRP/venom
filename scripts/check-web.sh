#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-web.sh [--lane quality|test|build|all]
EOF
}

fail() {
  echo "ERROR: $*" >&2
  echo "RESULT: FAIL"
  exit 1
}

lane="all"

while [[ $# -gt 0 ]]; do
  case "$1" in
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

[[ -f apps/web/package.json ]] || fail "missing apps/web/package.json"

run_quality() {
  (
    cd apps/web
    npm run check
  )
}

run_test() {
  (
    cd apps/web
    npm run test
  )
}

run_build() {
  (
    cd apps/web
    npm run build
  )
}

case "$lane" in
  quality)
    run_quality
    ;;
  test)
    run_test
    ;;
  build)
    run_build
    ;;
  all)
    run_quality
    run_test
    run_build
    ;;
  *)
    fail "unsupported web lane: $lane"
    ;;
esac

echo "RESULT: PASS"
