#!/usr/bin/env bash
# macOS ARM64 向け LLVM リンク検証スクリプト。

set -euo pipefail

DEFAULT_SYMBOLS=(
  "LLVMConstStringInContext2"
  "LLVMPositionBuilderBeforeInstrAndDbgRecords"
  "LLVMPrintDbgRecordToString"
)
DEFAULT_BINARIES=(
  "src/main.exe"
  "tests/test_llvm_array_access.exe"
)

REPORT_PATH=""
JSON_REPORT_PATH=""
FAIL_ON_MISSING_BINARY=0
declare -a SYMBOLS=("${DEFAULT_SYMBOLS[@]}")
declare -a BINARIES=("${DEFAULT_BINARIES[@]}")
declare -a REPORT_LINES=()
declare -a WARNING_LINES=()

usage() {
  cat <<'USAGE'
Usage: scripts/ci-verify-llvm-link.sh [options]

Options:
  --binary <path>            解析対象のバイナリパス（繰り返し指定可）
  --expected-symbol <name>   検証する LLVM シンボル名（繰り返し指定可）
  --report <path>            検証結果を Markdown で出力
  --json-report <path>       検証結果を JSON で出力
  --reset-binaries           既定バイナリ一覧をクリアして再指定する
  --fail-on-missing-binary   バイナリが存在しない場合にエラー終了
  -h, --help                 このメッセージを表示
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)
      shift
      BINARIES+=("${1:-}")
      shift
      ;;
    --reset-binaries)
      BINARIES=()
      shift
      ;;
    --expected-symbol)
      shift
      SYMBOLS+=("${1:-}")
      shift
      ;;
    --report)
      shift
      REPORT_PATH="${1:-}"
      shift
      ;;
    --json-report)
      shift
      JSON_REPORT_PATH="${1:-}"
      shift
      ;;
    --fail-on-missing-binary)
      FAIL_ON_MISSING_BINARY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

log_report() {
  if [[ $# -gt 0 ]]; then
    REPORT_LINES+=("$*")
  fi
}

OPAM_PREFIX="$(opam var prefix 2>/dev/null || true)"
if [[ -z "$OPAM_PREFIX" ]]; then
  echo "LLVM_LINK_ERROR: opam 環境を取得できませんでした。" >&2
  exit 2
fi

OPAM_LIB="$(opam var lib 2>/dev/null || true)"
if [[ -z "$OPAM_LIB" ]]; then
  echo "LLVM_LINK_ERROR: opam lib ディレクトリを取得できませんでした。" >&2
  exit 2
fi

OPAM_LLVM_LIB="${OPAM_LIB}/llvm"
if [[ ! -d "$OPAM_LLVM_LIB" ]]; then
  echo "LLVM_LINK_ERROR: LLVM ライブラリディレクトリが見つかりません: ${OPAM_LLVM_LIB}" >&2
  exit 2
fi

BREW_LLVM_PREFIX="$(brew --prefix llvm@18 2>/dev/null || true)"
if [[ -n "$BREW_LLVM_PREFIX" ]]; then
  BREW_LLVM_PREFIX="$(cd "$BREW_LLVM_PREFIX" && pwd)"
fi

llvm_version="$(opam exec -- llvm-config --version 2>/dev/null || true)"
llvm_libdir="$(opam exec -- llvm-config --libdir 2>/dev/null || true)"
if [[ -z "$llvm_version" || -z "$llvm_libdir" ]]; then
  echo "LLVM_LINK_ERROR: opam llvm-config の実行に失敗しました。" >&2
  exit 2
fi

log_report "# LLVM リンク検証レポート"
log_report ""
log_report "- LLVM version: ${llvm_version}"
log_report "- llvm-config --libdir: ${llvm_libdir}"
log_report "- opam LLVM libdir: ${OPAM_LLVM_LIB}"
if [[ -n "$BREW_LLVM_PREFIX" ]]; then
  log_report "- Homebrew llvm@18 prefix: ${BREW_LLVM_PREFIX}"
fi
log_report ""

if [[ "${llvm_libdir}" != "${OPAM_LLVM_LIB}"* ]]; then
  echo "LLVM_LINK_MISMATCH: llvm-config --libdir が opam スイッチ外を指しています (${llvm_libdir})." >&2
  exit 1
fi

detect_library_file() {
  local base_name="$1"
  local dylib_path="${OPAM_LLVM_LIB}/${base_name}.dylib"
  local static_path="${OPAM_LLVM_LIB}/${base_name}.a"

  if [[ -f "$dylib_path" ]]; then
    printf '%s\n' "$dylib_path"
    return 0
  fi

  if [[ -f "$static_path" ]]; then
    printf '%s\n' "$static_path"
    return 0
  fi

  return 1
}

LIBLLVM_CORE="$(detect_library_file "libLLVMCore")"
if [[ -z "$LIBLLVM_CORE" ]]; then
  echo "LLVM_LINK_ERROR: ${OPAM_LLVM_LIB} 内に libLLVMCore.{dylib,a} が見つかりません。" >&2
  exit 1
fi

missing_symbols=()
missing_symbol_file="$(mktemp)"
binary_result_file="$(mktemp)"
trap 'rm -f "$missing_symbol_file" "$binary_result_file"' EXIT

log_report "## シンボル検証"
log_report ""
log_report "- 検証対象: ${LIBLLVM_CORE}"
for symbol in "${SYMBOLS[@]}"; do
  if nm -gU "$LIBLLVM_CORE" 2>/dev/null | grep -q -- "$symbol"; then
    log_report "- OK: ${symbol}"
  else
    missing_symbols+=("$symbol")
    printf '%s\n' "$symbol" >>"$missing_symbol_file"
    log_report "- NG: ${symbol}"
    echo "LLVM_SYMBOL_MISSING:${LIBLLVM_CORE}:${symbol}" >&2
  fi
done
log_report ""

normalize_path() {
  local raw="$1"
  local trimmed="${raw#"${raw%%[![:space:]]*}"}"
  trimmed="${trimmed%% (*}"
  printf '%s\n' "$trimmed"
}

append_binary_result() {
  local binary="$1"
  local status="$2"
  local issues="$3"
  printf '%s\t%s\t%s\n' "$binary" "$status" "$issues" >>"$binary_result_file"
}

check_binary_otool() {
  local binary="$1"

  if [[ ! -f "$binary" ]]; then
    local message="バイナリが見つかりません。"
    local issue_record=":::${message}"
    if (( FAIL_ON_MISSING_BINARY )); then
      echo "LLVM_LINK_WARN:${binary}:${message}" >&2
      append_binary_result "$binary" "failure" "$issue_record"
      return 2
    else
      echo "LLVM_LINK_WARN:${binary}:${message}" >&2
      log_report "- WARN: ${binary} が存在しません。"
      append_binary_result "$binary" "skipped" "$issue_record"
      return 0
    fi
  fi

  local mismatch=0
  local llvm_ok=0
  local -a issues=()
  mapfile -t lines < <(otool -L "$binary" | tail -n +2 || true)

  for raw in "${lines[@]}"; do
    local path
    path="$(normalize_path "$raw")"
    if [[ "$path" != *libLLVM*.dylib ]]; then
      if [[ "$path" == *libunwind*.dylib && "$path" == *"/Cellar/llvm@18/"* ]]; then
        local msg="Cellar パスが検出されました"
        echo "LLVM_LINK_MISMATCH:${binary}:${path}:${msg}" >&2
        WARNING_LINES+=("LLVM_LINK_MISMATCH:${binary}:${path}:${msg}")
        mismatch=1
        issues+=("${path}:::${msg}")
      fi
      continue
    fi

    if [[ "$path" == @* ]]; then
      llvm_ok=1
      continue
    fi

    if [[ "$path" == "${OPAM_LLVM_LIB}"* ]]; then
      llvm_ok=1
      continue
    fi

    if [[ -n "$BREW_LLVM_PREFIX" && "$path" == "${BREW_LLVM_PREFIX}"* ]]; then
      llvm_ok=1
      continue
    fi

    local msg="許可されていない libLLVM パスです"
    echo "LLVM_LINK_MISMATCH:${binary}:${path}:${msg}" >&2
    WARNING_LINES+=("LLVM_LINK_MISMATCH:${binary}:${path}:${msg}")
    mismatch=1
    issues+=("${path}:::${msg}")
  done

  if [[ $llvm_ok -eq 0 ]]; then
    local msg="libLLVM のリンク先が確認できません"
    echo "LLVM_LINK_MISMATCH:${binary}:${msg}" >&2
    WARNING_LINES+=("LLVM_LINK_MISMATCH:${binary}:${msg}")
    mismatch=1
    issues+=(":::${msg}")
  fi

  if [[ $mismatch -ne 0 ]]; then
    local issue_join=""
    if [[ ${#issues[@]} -gt 0 ]]; then
      issue_join="${issues[0]}"
      if [[ ${#issues[@]} -gt 1 ]]; then
        for (( idx=1; idx<${#issues[@]}; idx++ )); do
          issue_join+="||${issues[idx]}"
        done
      fi
    fi
    append_binary_result "$binary" "failure" "$issue_join"
    return 1
  fi

  append_binary_result "$binary" "success" ""
  log_report "- OK: ${binary}"
  return 0
}

log_report "## バイナリ依存関係チェック"
log_report ""

binary_errors=0
for binary in "${BINARIES[@]}"; do
  if ! check_binary_otool "$binary"; then
    ((binary_errors++))
  fi
done

if [[ -n "$REPORT_PATH" ]]; then
  mkdir -p "$(dirname "$REPORT_PATH")"
  {
    for line in "${REPORT_LINES[@]}"; do
      echo "${line}"
    done
  } >"$REPORT_PATH"
fi

overall_status="success"
if (( ${#missing_symbols[@]} > 0 )) || (( binary_errors > 0 )); then
  overall_status="failure"
fi

if [[ -n "$JSON_REPORT_PATH" ]]; then
  mkdir -p "$(dirname "$JSON_REPORT_PATH")"
  export LLVM_VERIFY_VERSION="$llvm_version"
  export LLVM_VERIFY_LIBDIR="$llvm_libdir"
  export LLVM_VERIFY_OPAM_LLVM_LIB="$OPAM_LLVM_LIB"
  export LLVM_VERIFY_LIBLLVM_CORE="$LIBLLVM_CORE"
  export LLVM_VERIFY_STATUS="$overall_status"
  export LLVM_VERIFY_BINARY_FILE="$binary_result_file"
  export LLVM_VERIFY_MISSING_FILE="$missing_symbol_file"
  python3 <<'PY' "$JSON_REPORT_PATH"
import json
import os
import sys
from pathlib import Path

report_path = Path(sys.argv[1])
binary_file = Path(os.environ.get("LLVM_VERIFY_BINARY_FILE", ""))
missing_file = Path(os.environ.get("LLVM_VERIFY_MISSING_FILE", ""))

def load_missing(path: Path) -> list[str]:
    if not path.exists():
        return []
    return [
        line.strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]

def load_binaries(path: Path) -> list[dict]:
    entries: list[dict] = []
    if not path.exists():
        return entries
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        parts = line.split("\t")
        binary = parts[0]
        status = parts[1] if len(parts) > 1 else "unknown"
        issues_raw = parts[2] if len(parts) > 2 else ""
        issues: list[dict] = []
        if issues_raw:
            for chunk in issues_raw.split("||"):
                if not chunk:
                    continue
                if ":::" in chunk:
                    path_part, message = chunk.split(":::", 1)
                    entry: dict = {"issue": message}
                    if path_part:
                        entry["path"] = path_part
                    issues.append(entry)
                else:
                    issues.append({"issue": chunk})
        entries.append(
            {
                "path": binary,
                "status": status,
                "issues": issues,
            }
        )
    return entries

payload = {
    "metric": "llvm.link",
    "status": os.environ.get("LLVM_VERIFY_STATUS", "unknown"),
    "llvm": {
        "version": os.environ.get("LLVM_VERIFY_VERSION"),
        "libdir": os.environ.get("LLVM_VERIFY_LIBDIR"),
        "opam_libdir": os.environ.get("LLVM_VERIFY_OPAM_LLVM_LIB"),
        "libllvm_core": os.environ.get("LLVM_VERIFY_LIBLLVM_CORE"),
    },
    "missing_symbols": load_missing(missing_file),
    "binaries": load_binaries(binary_file),
}

report_path.write_text(
    json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
    encoding="utf-8",
)
PY
fi

if (( ${#missing_symbols[@]} > 0 )) || (( binary_errors > 0 )); then
  exit 1
fi

exit 0
