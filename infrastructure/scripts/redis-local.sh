#!/usr/bin/env bash
# Local Redis for Notifi development.
#
# Manual build (2026-09-16): `brew install redis` stalled on an unreachable
# GNU mirror, so Redis 8.10.1 was compiled from source and installed to
# ~/tools/redis (core binaries only: redis-server + redis-cli). Localhost-only,
# no password, no persistence — the server only PINGs Redis for /readyz
# (see src/api/handlers.rs), so there is no data worth keeping.
#
# Usage: scripts/redis-local.sh {start|stop|status|restart}
set -euo pipefail

REDIS_BIN="$HOME/tools/redis/bin/redis-server"
REDIS_CLI="$HOME/tools/redis/bin/redis-cli"
DATA_DIR="$HOME/tools/redis/data"
LOG_FILE="$HOME/tools/redis/redis.log"
PORT=6379

running() {
    "$REDIS_CLI" -h 127.0.0.1 -p "$PORT" ping >/dev/null 2>&1
}

case "${1:-status}" in
start)
    if running; then
        echo "redis already running on 127.0.0.1:$PORT"
        exit 0
    fi
    "$REDIS_BIN" --port "$PORT" --bind 127.0.0.1 --daemonize yes \
        --dir "$DATA_DIR" --save '' --appendonly no --logfile "$LOG_FILE"
    sleep 1
    running && echo "redis started on 127.0.0.1:$PORT"
    ;;
stop)
    if running; then
        "$REDIS_CLI" -h 127.0.0.1 -p "$PORT" shutdown nosave
        echo "redis stopped"
    else
        echo "redis not running"
    fi
    ;;
restart)
    "$0" stop
    sleep 1
    "$0" start
    ;;
status)
    if running; then
        echo "redis running on 127.0.0.1:$PORT"
    else
        echo "redis not running (start with: $0 start)"
        exit 1
    fi
    ;;
*)
    echo "usage: $0 {start|stop|status|restart}" >&2
    exit 2
    ;;
esac
