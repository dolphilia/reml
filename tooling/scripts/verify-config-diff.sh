#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
MANIFEST_PATH="${REPO_ROOT}/examples/core_config/reml.toml"
SCHEMA_PATH="${REPO_ROOT}/examples/core_config/cli/schema.json"
OLD_CONFIG="${REPO_ROOT}/examples/core_config/cli/config_old.json"
NEW_CONFIG="${REPO_ROOT}/examples/core_config/cli/config_new.json"
GOLDEN_PATH="${REPO_ROOT}/examples/core_config/cli/diff.expected.json"
OUTPUT_PATH="${REPO_ROOT}/tmp/config-diff-report.json"
UPDATE_GOLDEN=${UPDATE_GOLDEN:-0}

cleanup() {
  rm -f "${OUTPUT_PATH}"
}
trap cleanup EXIT

mkdir -p "${REPO_ROOT}/tmp"

echo "[verify-config-diff] Generating diff report from remlc"
cargo run --manifest-path "${REPO_ROOT}/compiler/frontend/Cargo.toml" \
  --bin remlc --quiet -- \
  config diff "${OLD_CONFIG}" "${NEW_CONFIG}" --format json \
  > "${OUTPUT_PATH}"

if [[ "${UPDATE_GOLDEN}" == "1" ]]; then
  mv "${OUTPUT_PATH}" "${GOLDEN_PATH}"
  echo "[verify-config-diff] Golden updated at ${GOLDEN_PATH}"
  exit 0
fi

if diff -u "${GOLDEN_PATH}" "${OUTPUT_PATH}"; then
  echo "[verify-config-diff] Config diff output matches golden"
else
  echo "[verify-config-diff] Detected diff mismatch. Inspect ${OUTPUT_PATH}" >&2
  exit 1
fi
