#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "ERROR: $*" >&2
  echo "RESULT: FAIL"
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

wait_for_url() {
  local url="$1"
  local label="$2"
  local attempt

  for attempt in $(seq 1 60); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  fail "${label} did not become ready at ${url}"
}

require_cmd cargo
require_cmd npm
require_cmd curl

[[ -f apps/web/package.json ]] || fail "missing apps/web/package.json"
[[ -f apps/web/playwright.config.ts ]] || fail "missing apps/web/playwright.config.ts"

api_port="${VENOM_E2E_API_PORT:-3300}"
web_port="${VENOM_E2E_WEB_PORT:-4173}"
api_url="http://127.0.0.1:${api_port}"
web_url="http://127.0.0.1:${web_port}"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/venom-web-e2e.XXXXXX")"
state_path="${tmp_dir}/state.jsonl"
runtime_path="${tmp_dir}/runtime.jsonl"
api_log="${tmp_dir}/api.log"
web_log="${tmp_dir}/web.log"

cleanup() {
  local exit_code=$?
  if [[ -n "${web_pid:-}" ]]; then
    kill "${web_pid}" >/dev/null 2>&1 || true
    wait "${web_pid}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${api_pid:-}" ]]; then
    kill "${api_pid}" >/dev/null 2>&1 || true
    wait "${api_pid}" >/dev/null 2>&1 || true
  fi
  rm -rf "${tmp_dir}"
  if [[ $exit_code -ne 0 ]]; then
    [[ -f "${api_log}" ]] && cat "${api_log}" >&2
    [[ -f "${web_log}" ]] && cat "${web_log}" >&2
  fi
  exit $exit_code
}

trap cleanup EXIT

VENOM_STATE_PATH="${state_path}" \
VENOM_RUNTIME_PATH="${runtime_path}" \
VENOM_API_BIND="127.0.0.1:${api_port}" \
cargo run -p venom-api >"${api_log}" 2>&1 &
api_pid=$!

wait_for_url "${api_url}/health" "api"

VITE_API_TARGET="${api_url}" \
npm --prefix apps/web run dev -- --host 127.0.0.1 --port "${web_port}" >"${web_log}" 2>&1 &
web_pid=$!

wait_for_url "${web_url}/findings" "web"

(
  cd apps/web
  PLAYWRIGHT_BASE_URL="${web_url}" npm run e2e
)

echo "RESULT: PASS"
