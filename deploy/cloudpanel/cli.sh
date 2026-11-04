#!/usr/bin/env bash
set -euo pipefail
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
set -a
[[ -f "$APP_DIR/.env" ]] && source "$APP_DIR/.env"
set +a
BIN_PATH="${MANIFESTADOR_NFE_BINARY:-$APP_DIR/bin/manifestador_nfe}"
if [[ "$BIN_PATH" != /* ]]; then
  BIN_PATH="$APP_DIR/${BIN_PATH#./}"
fi
export MANIFESTADOR_NFE_ENV_FILE="${MANIFESTADOR_NFE_ENV_FILE:-$APP_DIR/.env}"
export MANIFESTADOR_NFE_WEB_DIST_DIR="${MANIFESTADOR_NFE_WEB_DIST_DIR:-$APP_DIR/dist}"
export MANIFESTADOR_NFE_DATA_DIR="${MANIFESTADOR_NFE_DATA_DIR:-$APP_DIR/data}"
"$BIN_PATH" --mode=cli --data-dir "$MANIFESTADOR_NFE_DATA_DIR" "$@"
