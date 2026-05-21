#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "ERROR: $*" >&2
  echo "RESULT: FAIL"
  exit 1
}

skip() {
  fail "$*"
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

wait_for_url() {
  local url="$1"
  local label="$2"
  local max_attempts="$3"
  local log_path="${4:-}"
  local attempt

  for attempt in $(seq 1 "${max_attempts}"); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    if [[ -n "${log_path}" ]] && [[ -f "${log_path}" ]] && grep -q "Operation not permitted" "${log_path}"; then
      skip "${label} could not bind a local port in the current sandbox"
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
api_ready_attempts="${VENOM_E2E_API_READY_ATTEMPTS:-120}"
web_ready_attempts="${VENOM_E2E_WEB_READY_ATTEMPTS:-60}"
api_url="http://127.0.0.1:${api_port}"
web_url="http://127.0.0.1:${web_port}"
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/venom-web-e2e.XXXXXX")"
state_path="${tmp_dir}/state.jsonl"
runtime_path="${tmp_dir}/runtime.jsonl"
api_log="${tmp_dir}/api.log"
web_log="${tmp_dir}/web.log"
e2e_log="${tmp_dir}/e2e.log"

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

skip_if_bind_forbidden() {
  local pid="$1"
  local log_path="$2"
  local label="$3"

  if kill -0 "${pid}" >/dev/null 2>&1; then
    return 0
  fi

  if [[ -f "${log_path}" ]] && grep -q "Operation not permitted" "${log_path}"; then
    skip "${label} could not bind a local port in the current sandbox"
  fi

  return 0
}

skip_if_browser_forbidden() {
  local log_path="$1"

  [[ -f "${log_path}" ]] || return 0

  if grep -Eq \
    "bootstrap_check_in .* Permission denied \(1100\)|MachPortRendezvousServer|browserType\.launch: Target page, context or browser has been closed" \
    "${log_path}"; then
    skip "playwright browser launch is not permitted in the current sandbox"
  fi
}

VENOM_STATE_PATH="${state_path}" \
VENOM_RUNTIME_PATH="${runtime_path}" \
VENOM_API_BIND="127.0.0.1:${api_port}" \
cargo run -p venom-api >"${api_log}" 2>&1 &
api_pid=$!

sleep 1
skip_if_bind_forbidden "${api_pid}" "${api_log}" "api"
wait_for_url "${api_url}/health" "api" "${api_ready_attempts}" "${api_log}"

VITE_API_TARGET="${api_url}" \
npm --prefix apps/web run dev -- --host 127.0.0.1 --port "${web_port}" >"${web_log}" 2>&1 &
web_pid=$!

sleep 1
skip_if_bind_forbidden "${web_pid}" "${web_log}" "web"
wait_for_url "${web_url}/findings" "web" "${web_ready_attempts}" "${web_log}"

if ! (
  cd apps/web
  PLAYWRIGHT_BASE_URL="${web_url}" npm run e2e >"${e2e_log}" 2>&1
); then
  skip_if_browser_forbidden "${e2e_log}"
  cat "${e2e_log}" >&2
  fail "web e2e failed"
fi

echo "RESULT: PASS"
