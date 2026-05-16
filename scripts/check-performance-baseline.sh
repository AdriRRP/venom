#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-performance-baseline.sh [--bench hot_paths]
EOF
}

fail() {
  echo "ERROR: $*" >&2
  echo "RESULT: FAIL"
  exit 1
}

bench_name="hot_paths"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bench)
      [[ $# -ge 2 ]] || fail "missing value for --bench"
      bench_name="$2"
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

case "$bench_name" in
  hot_paths)
    ;;
  *)
    fail "unsupported bench: $bench_name"
    ;;
esac

cargo bench -p venom-domain --bench "$bench_name" -- --noplot

echo "RESULT: PASS"
