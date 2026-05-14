#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-git-discipline.sh --mode slice|wave [--wave WXX-<slug>]
EOF
}

fail() {
  echo "ERROR: $*" >&2
  echo "RESULT: FAIL"
  exit 1
}

mode=""
wave=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      [[ $# -ge 2 ]] || fail "missing value for --mode"
      mode="$2"
      shift 2
      ;;
    --wave)
      [[ $# -ge 2 ]] || fail "missing value for --wave"
      wave="$2"
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

[[ -n "$mode" ]] || fail "--mode is required"
[[ "$mode" == "slice" || "$mode" == "wave" ]] || fail "unsupported mode: $mode"

git rev-parse --show-toplevel >/dev/null 2>&1 || fail "current directory is not a git repository"

if [[ "$mode" == "slice" ]]; then
  echo "RESULT: PASS"
  exit 0
fi

git diff --quiet || fail "wave gate requires a clean working tree"
git diff --cached --quiet || fail "wave gate requires no staged-but-uncommitted changes"

if [[ -n "$wave" ]]; then
  doc="docs/waves/${wave}.md"
  [[ -f "$doc" ]] || fail "missing wave doc: $doc"
  status="$(sed -n 's/^Status: `\(.*\)`$/\1/p' "$doc" | head -n 1)"
  [[ "$status" == "done" ]] || fail "wave gate requires status \`done\`, got: ${status:-missing}"
fi

echo "RESULT: PASS"
