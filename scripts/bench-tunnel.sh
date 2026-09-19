#!/usr/bin/env bash
# Throughput / first-byte benchmark for NexusGate tunnel.
# Usage (on a host that can reach the gateway):
#   GATEWAY=http://127.0.0.1:8080 SIZE_MB=64 ./scripts/bench-tunnel.sh
set -euo pipefail

GATEWAY="${GATEWAY:-http://127.0.0.1:8080}"
SIZE_MB="${SIZE_MB:-32}"
ROUNDS="${ROUNDS:-3}"
BACKEND_PORT="${BACKEND_PORT:-18080}"

echo "== NexusGate tunnel bench =="
echo "gateway=$GATEWAY size=${SIZE_MB}MiB rounds=$ROUNDS"

# Optional: spin a local backend that serves SIZE_MB of zeros if BACKEND=1
if [[ "${BACKEND:-0}" == "1" ]]; then
  python3 - <<PY &
from http.server import BaseHTTPRequestHandler, HTTPServer
SIZE = ${SIZE_MB} * 1024 * 1024
PAYLOAD = b"0" * (1024 * 1024)

class H(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Length", str(SIZE))
        self.end_headers()
        left = SIZE
        while left > 0:
            chunk = PAYLOAD if left >= len(PAYLOAD) else PAYLOAD[:left]
            self.wfile.write(chunk)
            left -= len(chunk)
    def log_message(self, *a):
        pass

HTTPServer(("127.0.0.1", ${BACKEND_PORT}), H).serve_forever()
PY
  BACKEND_PID=$!
  trap 'kill $BACKEND_PID 2>/dev/null || true' EXIT
  sleep 0.3
fi

for i in $(seq 1 "$ROUNDS"); do
  start=$(date +%s%N)
  # First byte via curl write-out; full body discarded to /dev/null
  out=$(curl -fsS -o /dev/null -w "ttfb_s=%{time_starttransfer} total_s=%{time_total} size=%{size_download} speed=%{speed_download}" \
    "$GATEWAY/" || echo "FAIL")
  end=$(date +%s%N)
  wall_ms=$(( (end - start) / 1000000 ))
  echo "[round $i] wall_ms=$wall_ms $out"
done

echo "done. Compare direct vs hub-relay via server logs (mode=direct/tcp vs hub-relay)."
