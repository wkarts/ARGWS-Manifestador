#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "O build iOS exige macOS com Xcode." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="${1:-${ROOT_DIR}/release/ios-app}"
GEN_DIR="${ROOT_DIR}/src-tauri/gen/apple"
mkdir -p "${OUTPUT_DIR}"
OUTPUT_DIR="$(cd "${OUTPUT_DIR}" && pwd)"

if [[ "${OUTPUT_DIR}" == "/" || "${OUTPUT_DIR}" == "${ROOT_DIR}" ]]; then
  echo "Diretório de saída iOS inseguro: ${OUTPUT_DIR}" >&2
  exit 1
fi

# O job só pode consumir bundles produzidos nesta execução.
find "${OUTPUT_DIR}" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
rm -rf "${GEN_DIR}/build"
LOG_FILE="${OUTPUT_DIR}/tauri-ios-build.log"
BUILD_MARKER="${OUTPUT_DIR}/.build-started-at"
touch "${BUILD_MARKER}"

cd "${ROOT_DIR}"
npm run tauri:ios:init

# O Tauri valida bundle.resources antes de executar beforeBuildCommand no fluxo mobile.
# Garanta que o frontend e a cópia src-tauri/dist existam também no uso direto deste script.
if [[ ! -f "${ROOT_DIR}/dist/index.html" ]]; then
  npm run build:web
fi
node ./scripts/prepare-tauri-resources.mjs

find_device_app() {
  find "${GEN_DIR}" "${OUTPUT_DIR}" "${ROOT_DIR}/src-tauri/target/aarch64-apple-ios" \
    -type f -path '*.app/Info.plist' -newer "${BUILD_MARKER}" \
    ! -path '*iphonesimulator*' \
    ! -path '*-iphonesimulator/*' \
    2>/dev/null \
    | while IFS= read -r info_plist; do
        candidate="${info_plist%/Info.plist}"
        if [[ -d "${candidate}" ]]; then
          printf '%s\n' "${candidate}"
        fi
      done \
    | LC_ALL=C sort \
    | head -n 1
}

# O processo pai do Tauri mantém o servidor de opções usado pelo build phase
# `ios xcode-script`. Por isso, o archive precisa permanecer sob o comando
# oficial `tauri ios build`; executar xcodebuild diretamente quebra esse canal.
set +e
CODE_SIGNING_ALLOWED=NO \
CODE_SIGNING_REQUIRED=NO \
CODE_SIGN_IDENTITY="" \
DEVELOPMENT_TEAM="" \
PROVISIONING_PROFILE_SPECIFIER="" \
npm run tauri -- ios build \
  --features mobile \
  --target aarch64 \
  --ci \
  --no-sign \
  --archive-only \
  2>&1 | tee "${LOG_FILE}"
TAURI_STATUS=${PIPESTATUS[0]}
set -e

if [[ ${TAURI_STATUS} -ne 0 ]]; then
  echo "O build Tauri iOS não assinado falhou. Consulte ${LOG_FILE}." >&2
  exit "${TAURI_STATUS}"
fi

APP_PATH="$(find_device_app || true)"
if [[ -z "${APP_PATH}" || ! -d "${APP_PATH}" ]]; then
  echo "O archive Tauri terminou sem erro, mas nenhum bundle .app para iphoneos foi localizado." >&2
  echo "Log do build: ${LOG_FILE}" >&2
  exit 1
fi

FINAL_APP="${OUTPUT_DIR}/$(basename "${APP_PATH}")"
rm -rf "${FINAL_APP}"
ditto "${APP_PATH}" "${FINAL_APP}"
rm -f "${BUILD_MARKER}"
printf '%s\n' "${FINAL_APP}" > "${OUTPUT_DIR}/app-path.txt"
echo "Bundle iOS ARM64 sem assinatura preparado em ${FINAL_APP}"
