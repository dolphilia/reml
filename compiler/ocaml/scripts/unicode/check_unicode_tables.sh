#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
UNICODE_VERSION="${REML_UNICODE_VERSION:-0-ascii}"
TMP_DIR="$PROJECT_ROOT/_build/unicode_xid_tables_check"
TARGET_DIR="$PROJECT_ROOT/src/lexer_tables"
GENERATOR="$PROJECT_ROOT/scripts/unicode/generate-xid-tables.py"
SOURCE_CACHE="$PROJECT_ROOT/third_party/unicode/$UNICODE_VERSION"

rm -rf "$TMP_DIR"
mkdir -p "$TMP_DIR"

python3 "$GENERATOR" \
  --source-cache "$SOURCE_CACHE" \
  --unicode-version "$UNICODE_VERSION" \
  --out-dir "$TMP_DIR"

python3 - "$TMP_DIR" "$TARGET_DIR" <<'PY'
import json
import pathlib
import sys

tmp_dir = pathlib.Path(sys.argv[1])
target_dir = pathlib.Path(sys.argv[2])

with (tmp_dir / "unicode_xid_manifest.json").open(encoding="utf-8") as f:
    tmp_manifest = json.load(f)
with (target_dir / "unicode_xid_manifest.json").open(encoding="utf-8") as f:
    target_manifest = json.load(f)

tmp_manifest.pop("generated_at", None)
target_manifest.pop("generated_at", None)

if tmp_manifest != target_manifest:
    import difflib
    from pprint import pformat
    tmp_dump = json.dumps(tmp_manifest, ensure_ascii=False, sort_keys=True, indent=2)
    target_dump = json.dumps(target_manifest, ensure_ascii=False, sort_keys=True, indent=2)
    diff = difflib.unified_diff(
        target_dump.splitlines(), tmp_dump.splitlines(),
        fromfile="repo", tofile="generated", lineterm=""
    )
    for line in diff:
        print(line)
    sys.exit("manifest mismatch")
PY

diff -u "$TARGET_DIR/unicode_xid_tables.ml" "$TMP_DIR/unicode_xid_tables.ml"

echo "Unicode XID tables are up-to-date."
