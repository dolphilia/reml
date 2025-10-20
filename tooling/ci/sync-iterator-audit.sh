#!/usr/bin/env bash
# iterator stage 監査サマリー生成スクリプト

set -euo pipefail

METRICS_PATH=""
VERIFY_LOG_PATH=""
AUDIT_PATH=""
OUTPUT_PATH=""
declare -a MACOS_AUDIT_PATHS=()

print_usage() {
    cat <<'EOF'
使い方:
  tooling/ci/sync-iterator-audit.sh --metrics <PATH> --verify-log <PATH> [--audit <PATH>] [--output <PATH>]

オプション:
  --metrics <PATH>      collect-iterator-audit-metrics.py が生成した JSON
  --verify-log <PATH>   verify_llvm_ir.sh のログファイル
  --audit <PATH>        AuditEnvelope JSON (単一ファイルまたは JSON Lines)
  --output <PATH>       Markdown サマリー出力先（既定: reports/iterator-stage-summary.md）
  --macos-ffi-samples <PATH>
                        macOS arm64 FFI 監査ログ（複数指定可）
  -h, --help            このヘルプを表示

説明:
  iterator.stage.audit_pass_rate と LLVM 検証ログを突合し、Stage トレースの差分・欠落を確認します。
  Stage トレースの不整合や pass_rate < 1.0 の場合は非ゼロで終了します。
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
        --audit)
            shift || { echo "error: --audit の直後にパスを指定してください" >&2; exit 1; }
            AUDIT_PATH="$1"
            shift
            ;;
        --output)
            shift || { echo "error: --output の直後にパスを指定してください" >&2; exit 1; }
            OUTPUT_PATH="$1"
            shift
            ;;
        --macos-ffi-samples)
            shift || { echo "error: --macos-ffi-samples の直後にパスを指定してください" >&2; exit 1; }
            MACOS_AUDIT_PATHS+=("$1")
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        -*)
            echo "error: 不明なオプション: $1" >&2
            print_usage
            exit 1
            ;;
        *)
            echo "error: 位置引数はサポートしていません: $1" >&2
            print_usage
            exit 1
            ;;
    esac
done

if [[ -z "$METRICS_PATH" || -z "$VERIFY_LOG_PATH" ]]; then
    echo "error: --metrics と --verify-log は必須です" >&2
    print_usage
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

if [[ -n "$AUDIT_PATH" && ! -f "$AUDIT_PATH" ]]; then
    echo "error: AuditEnvelope ファイルが見つかりません: $AUDIT_PATH" >&2
    exit 1
fi

if [[ ${#MACOS_AUDIT_PATHS[@]} -gt 0 ]]; then
    for macos_audit in "${MACOS_AUDIT_PATHS[@]}"; do
        if [[ ! -f "$macos_audit" ]]; then
            echo "error: macOS FFI 監査ファイルが見つかりません: $macos_audit" >&2
            exit 1
        fi
    done
fi

if [[ -z "$OUTPUT_PATH" ]]; then
    OUTPUT_PATH="reports/iterator-stage-summary.md"
fi

if [[ "$OUTPUT_PATH" != "-" ]]; then
    mkdir -p "$(dirname "$OUTPUT_PATH")"
fi

PYTHON_OUTPUT="$(python3 - "$METRICS_PATH" "$VERIFY_LOG_PATH" "${AUDIT_PATH:-}" "${MACOS_AUDIT_PATHS[@]}" <<'PYCODE'
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

argv_iter = sys.argv[1:]
metrics_path = Path(argv_iter[0])
verify_log_path = Path(argv_iter[1])
audit_path = None
macos_audit_paths: List[Path] = []
if len(argv_iter) >= 3 and argv_iter[2]:
    audit_path = Path(argv_iter[2])
if len(argv_iter) > 3:
    macos_audit_paths = [Path(arg) for arg in argv_iter[3:] if arg]

def load_json(path: Path) -> Dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        try:
            return json.load(handle)
        except json.JSONDecodeError as exc:
            raise SystemExit(f"ERROR: JSON の解析に失敗しました ({path}): {exc}")


def load_audit_entries(path: Path) -> List[Dict[str, Any]]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise SystemExit(f"ERROR: 監査ファイルの読み込みに失敗しました: {exc}") from exc

    text = text.strip()
    if not text:
        return []

    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        entries: List[Dict[str, Any]] = []
        for line_no, line in enumerate(text.splitlines(), start=1):
            line = line.strip()
            if not line:
                continue
            try:
                entries.append(json.loads(line))
            except json.JSONDecodeError as exc:
                raise SystemExit(
                    f"ERROR: JSON Lines の解析に失敗しました ({path}:{line_no}): {exc}"
                )
        return entries

    if isinstance(data, list):
        return [entry for entry in data if isinstance(entry, dict)]
    if isinstance(data, dict):
        return [data]
    return []


def load_metrics(path: Path) -> Dict[str, Any]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
    except Exception as exc:
        raise SystemExit(f"ERROR: メトリクスJSONの読み込みに失敗しました: {exc}")


def load_verify_log(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except Exception as exc:
        raise SystemExit(f"ERROR: verify_llvm_ir ログの読み込みに失敗しました: {exc}")


metrics_data = load_metrics(metrics_path)
verify_log_text = load_verify_log(verify_log_path)

iterator_metrics: Dict[str, Any] = metrics_data
ffi_metrics: Optional[Dict[str, Any]] = None
ffi_platform_issue = False
macos_samples_issue = False

if isinstance(metrics_data.get("metrics"), list):
    for entry in metrics_data["metrics"]:
        if not isinstance(entry, dict):
            continue
        metric_name = entry.get("metric")
        if metric_name == "iterator.stage.audit_pass_rate":
            iterator_metrics = entry
        elif metric_name == "ffi_bridge.audit_pass_rate":
            ffi_metrics = entry

if ffi_metrics is None and isinstance(metrics_data.get("ffi_bridge"), dict):
    ffi_metrics = metrics_data["ffi_bridge"]

pass_rate_value: Any = iterator_metrics.get("pass_rate")
try:
    pass_rate_float = (
        float(pass_rate_value) if pass_rate_value is not None else None
    )
except (TypeError, ValueError):
    pass_rate_float = None

ffi_pass_rate_raw: Any = None
ffi_pass_rate_float: Optional[float] = None
if ffi_metrics is not None:
    ffi_pass_rate_raw = ffi_metrics.get("pass_rate")
    try:
        ffi_pass_rate_float = (
            float(ffi_pass_rate_raw)
            if ffi_pass_rate_raw is not None
            else None
        )
    except (TypeError, ValueError):
        ffi_pass_rate_float = None

if "検証成功" in verify_log_text or "Verification succeeded" in verify_log_text:
    log_status = "成功"
elif re.search(r"(検証失敗|Verification failed|エラー|失敗)", verify_log_text):
    log_status = "失敗"
else:
    log_status = "不明"

current_date = datetime.now(timezone.utc).strftime("%Y-%m-%d")

lines: List[str] = []
lines.append(f"### Iterator Stage Audit サマリー ({current_date})\n")
lines.append(f"- メトリクスファイル: `{metrics_path}`")
lines.append(f"- verify ログ: `{verify_log_path}` （判定: {log_status}）")
lines.append(f"- 指標: `{iterator_metrics.get('metric', 'iterator.stage.audit_pass_rate')}`")
lines.append(
    f"- 合計: {iterator_metrics.get('total', 0)}, 成功: {iterator_metrics.get('passed', 0)}, "
    f"失敗: {iterator_metrics.get('failed', 0)}, pass_rate: {pass_rate_value}"
)

sources: List[str] = iterator_metrics.get("sources", []) or []
if sources:
    lines.append(f"- 解析対象ファイル数: {len(sources)}")
    for src in sources:
        lines.append(f"  - `{src}`")

if ffi_metrics is not None:
    lines.append(
        f"- FFI ブリッジ指標: `{ffi_metrics.get('metric', 'ffi_bridge.audit_pass_rate')}`"
    )
    lines.append(
        f"  - 合計: {ffi_metrics.get('total', 0)}, 成功: {ffi_metrics.get('passed', 0)}, "
        f"失敗: {ffi_metrics.get('failed', 0)}, pass_rate: {ffi_pass_rate_raw}"
    )
    ffi_sources: List[str] = ffi_metrics.get("sources", []) or []
    if ffi_sources:
        lines.append(f"  - FFI 解析対象ファイル数: {len(ffi_sources)}")
        for src in ffi_sources:
            lines.append(f"    - `{src}`")

    platform_summary = ffi_metrics.get("platform_summary")
    if isinstance(platform_summary, dict) and platform_summary:
        lines.append("  - プラットフォーム別サマリー:")
        for platform in sorted(platform_summary.keys()):
            stats = platform_summary.get(platform) or {}
            total = int(stats.get("total", 0) or 0)
            ok = int(stats.get("ok", 0) or 0)
            failed = int(stats.get("failed", 0) or 0)
            lines.append(
                f"    - {platform}: total={total}, ok={ok}, failed={failed}"
            )
            if platform == "macos-arm64":
                if ok == 0 or failed > 0:
                    ffi_platform_issue = True
                    lines.append(
                        "      ⚠️ macOS (macos-arm64) で成功した監査ログが確認できません"
                    )
    else:
        lines.append("  - プラットフォーム別サマリー: (データなし)")
        ffi_platform_issue = True

if macos_audit_paths:
    lines.append("\n#### macOS FFI サンプル監査")

    def is_success_status(value: Optional[str]) -> bool:
        if value is None:
            return False
        return str(value) in {"ok", "wrap", "wrap_and_release"}

    for sample_path in macos_audit_paths:
        try:
            entries = load_audit_entries(sample_path)
        except SystemExit as exc:  # re-raise with context
            macos_samples_issue = True
            lines.append(f"- ❌ `{sample_path}`: 監査ログの読み込みに失敗しました ({exc})")
            continue

        if not entries:
            macos_samples_issue = True
            lines.append(f"- ❌ `{sample_path}`: 監査エントリが存在しません")
            continue

        found_success = False
        for entry in entries:
            metadata = {}
            if isinstance(entry, dict):
                if isinstance(entry.get("metadata"), dict):
                    metadata = entry["metadata"]
                else:
                    metadata = entry
            bridge = metadata.get("bridge") if isinstance(metadata, dict) else None
            if not isinstance(bridge, dict):
                continue
            platform = bridge.get("platform")
            status_value = bridge.get("status")
            if platform == "macos-arm64" and is_success_status(status_value):
                found_success = True
                break

        if found_success:
            lines.append(f"- ✅ `{sample_path}`: macos-arm64 の成功監査を確認")
        else:
            macos_samples_issue = True
            lines.append(
                f"- ⚠️ `{sample_path}`: macos-arm64 の成功監査が見つかりません"
            )

failures: List[Dict[str, Any]] = iterator_metrics.get("failures", []) or []
if failures:
    lines.append("\n#### 監査必須キーの欠落")
    for failure in failures:
        file = failure.get("file", "<unknown>")
        idx = failure.get("index", "?")
        missing = ", ".join(failure.get("missing", []))
        lines.append(f"- `{file}` (diagnostic #{idx}) → 欠落フィールド: {missing}")
else:
    lines.append("\n- 監査必須キー: すべて揃っています 🎉")

ffi_failures: List[Dict[str, Any]] = []
if ffi_metrics is not None:
    ffi_failures = ffi_metrics.get("failures", []) or []
    if ffi_failures:
        lines.append("\n#### FFI ブリッジ監査の欠落")
        for failure in ffi_failures:
            file = failure.get("file", "<unknown>")
            idx = failure.get("index", "?")
            missing = ", ".join(failure.get("missing", []))
            code = failure.get("code", "ffi.contract.*")
            status = failure.get("status", "unknown")
            platform = failure.get("platform", "<unknown>")
            lines.append(
                f"- `{file}` (diagnostic #{idx}, code={code}, status={status}, platform={platform}) → 欠落フィールド: {missing}"
            )
    else:
        lines.append("\n- FFI ブリッジ監査: すべて揃っています ✅")

stage_trace_missing = 0
stage_trace_source_missing = 0
stage_trace_mismatch = 0
stage_trace_entries: List[Dict[str, Any]] = []

def normalise_stage_trace(trace: Any) -> List[Dict[str, Any]]:
    result: List[Dict[str, Any]] = []
    if not isinstance(trace, list):
        return result
    for step in trace:
        if not isinstance(step, dict):
            continue
        result.append(
            {
                "source": step.get("source"),
                "stage": step.get("stage"),
                "capability": step.get("capability"),
                "note": step.get("note"),
            }
        )
    return result


def find_stage_by_keywords(trace: List[Dict[str, Any]], keywords: List[str]) -> Optional[Dict[str, Any]]:
    for step in trace:
        source = (step.get("source") or "").lower()
        if any(keyword in source for keyword in keywords):
            return step
    return None


if audit_path is not None:
    audit_entries = load_audit_entries(audit_path)
    if audit_entries:
        lines.append("\n#### Stage トレース検証")

    for index, entry in enumerate(audit_entries):
        metadata = {}
        if isinstance(entry, dict):
            if "metadata" in entry and isinstance(entry["metadata"], dict):
                metadata = entry["metadata"]
            else:
                metadata = entry

        category = entry.get("category") if isinstance(entry, dict) else None
        if not isinstance(category, str):
            category = ""
        if not category.startswith("effect.stage"):
            continue

        stage_trace_raw = metadata.get("stage_trace")
        trace = normalise_stage_trace(stage_trace_raw)

        if not trace:
            stage_trace_missing += 1
            stage_trace_entries.append(
                {
                    "index": index,
                    "status": "missing",
                    "detail": "stage_trace が存在しません",
                }
            )
            continue

        typer_step = find_stage_by_keywords(trace, ["typer"])
        runtime_step = find_stage_by_keywords(trace, ["runtime"])

        if typer_step is None or runtime_step is None:
            stage_trace_source_missing += 1
            stage_trace_entries.append(
                {
                    "index": index,
                    "status": "incomplete",
                    "detail": "typer/runtime の両方のステップが揃っていません",
                    "trace": trace,
                }
            )
            continue

        typer_stage = typer_step.get("stage")
        runtime_stage = runtime_step.get("stage")

        if typer_stage != runtime_stage:
            stage_trace_mismatch += 1
            stage_trace_entries.append(
                {
                    "index": index,
                    "status": "mismatch",
                    "typer_stage": typer_stage,
                    "runtime_stage": runtime_stage,
                    "trace": trace,
                }
            )
        else:
            stage_trace_entries.append(
                {
                    "index": index,
                    "status": "ok",
                    "stage": typer_stage,
                    "trace": trace,
                }
            )

    if audit_path is not None:
        lines.append(
            f"- トレース件数: {len(stage_trace_entries)}, "
            f"欠落: {stage_trace_missing}, "
            f"不足: {stage_trace_source_missing}, "
            f"差分: {stage_trace_mismatch}"
        )

    if stage_trace_entries:
        lines.append("")
        for entry in stage_trace_entries:
            status = entry["status"]
            idx = entry["index"]
            if status == "ok":
                lines.append(f"- ✅ trace#{idx}: stage={entry.get('stage')}")
            elif status == "missing":
                lines.append(f"- ❌ trace#{idx}: {entry['detail']}")
            elif status == "incomplete":
                lines.append(f"- ❌ trace#{idx}: {entry['detail']}")
            elif status == "mismatch":
                lines.append(
                    f"- ❌ trace#{idx}: typer={entry.get('typer_stage')} / "
                    f"runtime={entry.get('runtime_stage')}"
                )

exit_code = 0
if pass_rate_float is None or pass_rate_float < 1.0:
    exit_code = 1
if log_status == "失敗":
    exit_code = 1
if audit_path is not None and (stage_trace_missing > 0 or stage_trace_source_missing > 0 or stage_trace_mismatch > 0):
    exit_code = 1
if ffi_metrics is not None:
    if ffi_pass_rate_float is None and ffi_metrics.get("total", 0) > 0:
        exit_code = 1
    elif ffi_pass_rate_float is not None and ffi_pass_rate_float < 1.0:
        exit_code = 1
    if ffi_failures:
        exit_code = 1
    if ffi_platform_issue:
        exit_code = 1
if macos_samples_issue:
    exit_code = 1

markdown = "\n".join(lines).rstrip() + "\n"

print(markdown, end="")
print(f"STATUS:{exit_code}")
PYCODE
)"

STATUS_LINE="${PYTHON_OUTPUT##*STATUS:}"
OUTPUT_MARKDOWN="${PYTHON_OUTPUT%STATUS:*}"

EXIT_CODE="$(printf '%s' "${STATUS_LINE:-1}" | tr -d '[:space:]')"
OUTPUT_MARKDOWN="$(printf '%s' "$OUTPUT_MARKDOWN")"

if [[ "$OUTPUT_PATH" == "-" ]]; then
    printf "%s" "$OUTPUT_MARKDOWN"
else
    printf "%s" "$OUTPUT_MARKDOWN" >"$OUTPUT_PATH"
    echo "Audit summary written to $OUTPUT_PATH"
fi

exit "${EXIT_CODE:-1}"
