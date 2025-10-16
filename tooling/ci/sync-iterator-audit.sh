#!/usr/bin/env bash
# iterator 監査メトリクスと LLVM IR 検証ログの突合せスクリプト
# 使い方:
#   ./tooling/ci/sync-iterator-audit.sh \
#       --metrics tooling/ci/iterator-audit-metrics.json \
#       --verify-log artifacts/llvm-verify/verify.log \
#       --output tooling/ci/iterator-audit-summary.md

set -euo pipefail

METRICS_PATH=""
VERIFY_LOG_PATH=""
OUTPUT_PATH=""

usage() {
    cat <<EOF
使い方:
  $(basename "$0") --metrics <PATH> --verify-log <PATH> [--output <PATH>]

オプション:
  --metrics <PATH>      collect-iterator-audit-metrics.py が生成した JSON へのパス
  --verify-log <PATH>   verify_llvm_ir.sh のログ（標準出力を保存したファイル）
  --output <PATH>       Markdown サマリーを書き出す先（省略時は標準出力へ出力）
  -h, --help            このヘルプを表示
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --metrics)
            shift || { echo "error: --metrics の直後にパスを指定してください" >&2; exit 1; }
            METRICS_PATH="$1"
            shift
            ;;
        --verify-log)
            shift || { echo "error: --verify-log の直後にパスを指定してください" >&2; exit 1; }
            VERIFY_LOG_PATH="$1"
            shift
            ;;
        --output)
            shift || { echo "error: --output の直後にパスを指定してください" >&2; exit 1; }
            OUTPUT_PATH="$1"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "error: 不明なオプション: $1" >&2
            usage
            exit 1
            ;;
    esac
done

if [[ -z "$METRICS_PATH" || -z "$VERIFY_LOG_PATH" ]]; then
    echo "error: --metrics と --verify-log は必須です" >&2
    usage
    exit 1
fi

if [[ ! -f "$METRICS_PATH" ]]; then
    echo "error: メトリクスファイルが見つかりません: $METRICS_PATH" >&2
    exit 1
fi

if [[ ! -f "$VERIFY_LOG_PATH" ]]; then
    echo "error: verify_llvm_ir ログが見つかりません: $VERIFY_LOG_PATH" >&2
    exit 1
fi

PYTHON_OUTPUT="$(python3 - "$METRICS_PATH" "$VERIFY_LOG_PATH" "$OUTPUT_PATH" <<'PYCODE'
import json
import re
import sys
from pathlib import Path
from typing import Any, List
from datetime import datetime

metrics_path = Path(sys.argv[1])
verify_log_path = Path(sys.argv[2])
output_path = sys.argv[3] or ""

try:
    metrics_data = json.loads(metrics_path.read_text(encoding="utf-8"))
except Exception as exc:
    print(f"ERROR: メトリクスJSONの読み込みに失敗しました: {exc}")
    sys.exit(1)

try:
    verify_log_text = verify_log_path.read_text(encoding="utf-8", errors="replace")
except Exception as exc:
    print(f"ERROR: verify_llvm_ir ログの読み込みに失敗しました: {exc}")
    sys.exit(1)

pass_rate_value: Any = metrics_data.get("pass_rate")
try:
    pass_rate_float = float(pass_rate_value) if pass_rate_value is not None else None
except (TypeError, ValueError):
    pass_rate_float = None

if "検証成功" in verify_log_text:
    log_status = "成功"
elif re.search(r"(検証失敗|エラー|失敗)", verify_log_text):
    log_status = "失敗"
else:
    log_status = "不明"

sources: List[str] = metrics_data.get("sources", []) or []
failures: List[dict] = metrics_data.get("failures", []) or []

current_date = datetime.utcnow().strftime("%Y-%m-%d")

lines: List[str] = []
lines.append(f"### Iterator Stage Audit サマリー ({current_date})")
lines.append("")
lines.append(f"- メトリクスファイル: `{metrics_path}`")
lines.append(f"- verify ログ: `{verify_log_path}` （判定: {log_status}）")
lines.append(f"- 指標: `{metrics_data.get('metric', 'iterator.stage.audit_pass_rate')}`")
lines.append(
    f"- 合計: {metrics_data.get('total', 0)}, 成功: {metrics_data.get('passed', 0)}, "
    f"失敗: {metrics_data.get('failed', 0)}, pass_rate: {pass_rate_value}"
)

if sources:
    lines.append(f"- 解析対象: {len(sources)} 件")
    for src in sources:
        lines.append(f"  - `{src}`")

if failures:
    lines.append("")
    lines.append("#### 失敗詳細")
    for failure in failures:
        file = failure.get("file", "<unknown>")
        idx = failure.get("index", "?")
        missing = ", ".join(failure.get("missing", []))
        lines.append(f"- `{file}` (diagnostic #{idx}) → 欠落フィールド: {missing}")
else:
    lines.append("")
    lines.append("- 失敗ケース: なし 🎉")

markdown = "\n".join(lines) + "\n"

if output_path:
    Path(output_path).write_text(markdown, encoding="utf-8")
else:
    print(markdown, end="")

exit_code = 0
if pass_rate_float is None or pass_rate_float < 1.0:
    exit_code = 1
if log_status == "失敗":
    exit_code = 1

print(f"STATUS:{exit_code}")
PYCODE
)"

if [[ "$PYTHON_OUTPUT" == STATUS:* ]]; then
    EXIT_CODE="${PYTHON_OUTPUT#STATUS:}"
    OUTPUT_TEXT=""
else
    # 出力には Markdown + STATUS 行が含まれる
    OUTPUT_TEXT="${PYTHON_OUTPUT%STATUS:*}"
    EXIT_CODE="${PYTHON_OUTPUT##*STATUS:}"
fi

if [[ -z "$OUTPUT_PATH" ]]; then
    printf "%s" "$OUTPUT_TEXT"
fi

EXIT_CODE="$(echo "${EXIT_CODE:-1}" | tr -d '[:space:]')"

exit "${EXIT_CODE:-1}"
