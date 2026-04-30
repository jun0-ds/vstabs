#!/usr/bin/env bash
# Stop the two code-server instances spawned by start-code-servers.sh

PIDS=$(ps -ef | grep 'code-server' | grep -E '808[01]' | grep -v grep | awk '{print $2}')
if [ -z "${PIDS}" ]; then
  echo "no code-server on ports 808[01] running"
  exit 0
fi
echo "killing PIDs: ${PIDS}"
kill ${PIDS} 2>/dev/null || true
sleep 1
REMAINING=$(ss -tln 2>/dev/null | grep -E ':808[01] ' | wc -l)
if [ "${REMAINING}" -eq 0 ]; then
  echo "stopped, ports 8080/8081 free"
else
  echo "WARN: still listening — try: pkill -9 -f 'code-server'"
fi
