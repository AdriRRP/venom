#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/configure-github-required-checks.sh [--owner OWNER --repo REPO] [--mode dry-run|apply]

Environment:
  GITHUB_TOKEN or GH_TOKEN   Optional for --mode apply
EOF
}

fail() {
  echo "ERROR: $*" >&2
  echo "RESULT: FAIL"
  exit 1
}

owner=""
repo=""
mode="dry-run"
ruleset_name="venom-main-required-checks"
api_version="2026-03-10"
payload_path="infra/github/main-required-checks.ruleset.json"

github_token="${GITHUB_TOKEN:-${GH_TOKEN:-}}"

detect_repo_from_origin() {
  local remote_url path_part

  remote_url="$(git remote get-url origin 2>/dev/null || true)"
  [[ -n "$remote_url" ]] || return 1

  case "$remote_url" in
    git@github.com:*)
      path_part="${remote_url#git@github.com:}"
      ;;
    https://github.com/*)
      path_part="${remote_url#https://github.com/}"
      ;;
    *)
      return 1
      ;;
  esac

  path_part="${path_part%.git}"
  owner="${path_part%%/*}"
  repo="${path_part#*/}"

  [[ -n "$owner" && -n "$repo" && "$owner" != "$repo" ]]
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --owner)
      [[ $# -ge 2 ]] || fail "missing value for --owner"
      owner="$2"
      shift 2
      ;;
    --repo)
      [[ $# -ge 2 ]] || fail "missing value for --repo"
      repo="$2"
      shift 2
      ;;
    --mode)
      [[ $# -ge 2 ]] || fail "missing value for --mode"
      mode="$2"
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

if [[ -z "$owner" || -z "$repo" ]]; then
  detect_repo_from_origin || fail "--owner and --repo are required when origin cannot be resolved"
fi

[[ -f "$payload_path" ]] || fail "missing payload file: $payload_path"

payload="$(cat "$payload_path")"

if [[ "$mode" == "dry-run" ]]; then
  echo "OWNER: $owner"
  echo "REPO: $repo"
  echo "MODE: dry-run"
  echo "$payload"
  echo "RESULT: PASS"
  exit 0
fi

[[ "$mode" == "apply" ]] || fail "unsupported mode: $mode"

if [[ -n "$github_token" ]]; then
  headers=(
    -H "Accept: application/vnd.github+json"
    -H "Authorization: Bearer ${github_token}"
    -H "X-GitHub-Api-Version: ${api_version}"
  )

  rulesets_json="$(
    curl -fsSL \
      "${headers[@]}" \
      "https://api.github.com/repos/${owner}/${repo}/rulesets"
  )"
else
  command -v gh >/dev/null 2>&1 || fail "apply mode requires gh auth login or GITHUB_TOKEN/GH_TOKEN"
  gh auth status >/dev/null 2>&1 || fail "apply mode requires gh auth login or GITHUB_TOKEN/GH_TOKEN"

  set +e
  rulesets_json="$(
    gh api \
      -H "X-GitHub-Api-Version: ${api_version}" \
      "repos/${owner}/${repo}/rulesets" 2>&1
  )"
  status=$?
  set -e

  if [[ $status -ne 0 ]]; then
    echo "$rulesets_json"
    if grep -q "Upgrade to GitHub Pro or make this repository public" <<<"$rulesets_json"; then
      fail "rulesets are unavailable for this private repository on the current GitHub plan"
    fi
    fail "failed to read repository rulesets"
  fi
fi

ruleset_id="$(
  printf '%s' "$rulesets_json" | python3 -c '
import json
import sys

name = sys.argv[1]
rulesets = json.load(sys.stdin)
for item in rulesets:
    if item.get("name") == name:
        print(item["id"])
        break
' "$ruleset_name"
)"

if [[ -n "$ruleset_id" ]]; then
  method="PUT"
  api_path="repos/${owner}/${repo}/rulesets/${ruleset_id}"
else
  method="POST"
  api_path="repos/${owner}/${repo}/rulesets"
fi

if [[ -n "$github_token" ]]; then
  response="$(
    curl -fsSL \
      -X "$method" \
      "${headers[@]}" \
      "https://api.github.com/${api_path}" \
      -d "$payload"
  )"
else
  set +e
  response="$(
    gh api \
      -X "$method" \
      -H "X-GitHub-Api-Version: ${api_version}" \
      "$api_path" \
      --input "$payload_path" 2>&1
  )"
  status=$?
  set -e

  if [[ $status -ne 0 ]]; then
    echo "$response"
    if grep -q "Upgrade to GitHub Pro or make this repository public" <<<"$response"; then
      fail "rulesets are unavailable for this private repository on the current GitHub plan"
    fi
    fail "failed to apply repository ruleset"
  fi
fi

echo "$response"
echo "RESULT: PASS"
