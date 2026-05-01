#!/usr/bin/env bash
# Start two code-server instances for vstabs v0.1 iframe spike.
# Edit PROJECT_A / PROJECT_B to point at folders you actually have.
set -euo pipefail

PROJECT_A="${HOME}/sample-a"
PROJECT_B="${HOME}/sample-b"

CS_BIN="${HOME}/.local/bin/code-server"
LOG_DIR="/tmp/cs-spike"
mkdir -p "${LOG_DIR}"

if [ ! -x "${CS_BIN}" ]; then
  echo "ERROR: code-server not found at ${CS_BIN}"
  echo "Install: curl -fsSL https://code-server.dev/install.sh | sh -s -- --method standalone --prefix \$HOME/.local"
  exit 1
fi

# VS Code IPC env vars hijack code-server into "open in existing instance" mode.
# Strip them so each call truly starts a daemon.
spawn() {
  local port="$1" folder="$2" tag="$3"
  nohup env -u VSCODE_IPC_HOOK_CLI -u VSCODE_IPC_HOOK -u VSCODE_PID -u VSCODE_CWD \
    "${CS_BIN}" --bind-addr "127.0.0.1:${port}" --auth none "${folder}" \
    > "${LOG_DIR}/cs-${tag}.log" 2>&1 &
  disown
}

spawn 8080 "${PROJECT_A}" a
spawn 8081 "${PROJECT_B}" b

# Wait for both to bind
for i in 1 2 3 4 5 6 7 8 9 10; do
  sleep 1
  up_a=$(ss -tln 2>/dev/null | grep -c ':8080 ' || true)
  up_b=$(ss -tln 2>/dev/null | grep -c ':8081 ' || true)
  if [ "${up_a}" -ge 1 ] && [ "${up_b}" -ge 1 ]; then
    echo "OK — both up after ${i}s"
    break
  fi
done

ss -tln 2>/dev/null | grep -E ':808[01] ' || echo "WARN: one or both ports not listening"
echo
echo "  A: http://127.0.0.1:8080  (${PROJECT_A})"
echo "  B: http://127.0.0.1:8081  (${PROJECT_B})"
echo
echo "Open index.html in Chrome or Edge to test the tab model."
echo "Logs: ${LOG_DIR}/cs-{a,b}.log"
echo "Stop: bash stop-code-servers.sh"
