#!/usr/bin/env bash
# LLVM IR検証パイプライン（cross 対応版）
#
# このスクリプトは生成された LLVM IR を段階的に検証する:
# 1. llvm-as でビットコード生成 (.ll → .bc)
# 2. opt -verify で検証パス実行
# 3. llc でオブジェクト生成 (.bc → .o)
# 4. （任意）--cross 指定時に ld.lld で Linux x86_64 へリンクし、objcopy でストリップ
#
# 使い方:
#   ./scripts/verify_llvm_ir.sh [オプション] <input.ll>
#
# 主なオプション:
#   --target <TRIPLE>        ターゲットトリプル（x86_64-unknown-linux-gnu / x86_64-apple-darwin / arm64-apple-darwin）
#   --preset <NAME>          事前定義のサンプルセットを検証（例: darwin-arm64）
#   --cross                  x86_64-unknown-linux-gnu 向けクロスリンクを実行
#   --cross-prefix <TRIPLE>  ターゲットトリプル（既定: x86_64-unknown-linux-gnu）
#   --sysroot <PATH>         クロスリンクに使用する sysroot（既定: REML_TOOLCHAIN_HOME/sysroot）
#   --ld <PATH>              クロスリンク時に使用する ld.lld のパス
#   --objcopy <PATH>         objcopy のパス（省略時は <TRIPLE>-objcopy / llvm-objcopy を探索）
#   --output <PATH>          クロスリンク結果の出力先（既定: <input>.elf）
#   -h, --help               ヘルプを表示
#
# 終了コード:
#   0: 検証成功
#   1: 引数エラー
#   2: llvm-as 失敗
#   3: opt -verify 失敗
#   4: llc 失敗
#   5: クロスリンク失敗
#
# 参考:
# - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §6
# - docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §10

set -euo pipefail

# スクリプトとプリセットのルート
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -d "$SCRIPT_DIR/../tests/llvm-ir/presets" ]]; then
  PRESET_ROOT="$(cd "$SCRIPT_DIR/../tests/llvm-ir/presets" && pwd)"
else
  PRESET_ROOT=""
fi

# ========== 設定 ==========

LLVM_MIN_VERSION="18.0"

# llvm-as, opt, llc の候補（環境変数で上書き可能）
LLVM_AS_CANDIDATE="${LLVM_AS:-}"
OPT_CANDIDATE="${OPT:-}"
LLC_CANDIDATE="${LLC:-}"

# ターゲット関連
TARGET_TRIPLE=""
PRESET_NAME=""

# クロス関連デフォルト
CROSS_MODE=0
CROSS_PREFIX="x86_64-unknown-linux-gnu"
CROSS_SYSROOT="${CROSS_SYSROOT:-}"
CROSS_LD="${CROSS_LD:-}"
CROSS_OBJCOPY="${CROSS_OBJCOPY:-}"
CROSS_OUTPUT=""

# ========== ヘルパー ==========

usage() {
  sed -n '1,35p' "$0"
}

error() {
  echo "エラー: $*" >&2
  exit 1
}

warn() {
  echo "警告: $*" >&2
}

resolve_tool() {
  local env_value="$1"
  local base_name="$2"
  shift 2
  local suffixes=("$@")

  if [[ -n "$env_value" ]]; then
    if command -v "$env_value" >/dev/null 2>&1; then
      command -v "$env_value"
      return 0
    elif [[ -x "$env_value" ]]; then
      echo "$env_value"
      return 0
    else
      error "'$env_value' (環境変数指定) が実行可能ではありません。"
    fi
  fi

  local candidate_path
  if candidate_path=$(command -v "$base_name" 2>/dev/null); then
    echo "$candidate_path"
    return 0
  fi

  for suffix in "${suffixes[@]}"; do
    local candidate="${base_name}-${suffix}"
    if candidate_path=$(command -v "$candidate" 2>/dev/null); then
      echo "$candidate_path"
      return 0
    fi
  done

  return 1
}

find_first_existing() {
  for candidate in "$@"; do
    if [[ -f "$candidate" ]]; then
      echo "$candidate"
      return 0
    fi
  done
  return 1
}

check_llvm_version() {
  if ! command -v "$LLVM_AS" >/dev/null 2>&1; then
    error "llvm-as が見つかりません。LLVM 15+ をインストールしてください。"
  fi

  local version_output
  version_output=$("$LLVM_AS" --version 2>&1 | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -1 || true)

  if [[ -z "$version_output" ]]; then
    warn "LLVM バージョンを検出できませんでした。続行します..."
    return 0
  fi

  local major_version
  major_version=$(echo "$version_output" | cut -d. -f1)

  if (( major_version < 18 )); then
    error "LLVM $version_output が検出されましたが、LLVM 18+ が必要です。"
  fi

  echo "LLVM $version_output を使用します。"
}

resolve_ld() {
  if [[ -n "$CROSS_LD" ]]; then
    echo "$CROSS_LD"
    return 0
  fi

  if command -v "${CROSS_PREFIX}-ld" >/dev/null 2>&1; then
    echo "${CROSS_PREFIX}-ld"
    return 0
  fi

  if command -v ld.lld >/dev/null 2>&1; then
    echo "$(command -v ld.lld)"
    return 0
  fi

  return 1
}

resolve_objcopy() {
  if [[ -n "$CROSS_OBJCOPY" ]]; then
    echo "$CROSS_OBJCOPY"
    return 0
  fi

  if command -v "${CROSS_PREFIX}-objcopy" >/dev/null 2>&1; then
    echo "${CROSS_PREFIX}-objcopy"
    return 0
  fi

  if command -v llvm-objcopy >/dev/null 2>&1; then
    echo "$(command -v llvm-objcopy)"
    return 0
  fi

  return 1
}

resolve_preset() {
  local name="$1"
  if [[ -z "$PRESET_ROOT" ]]; then
    return 1
  fi
  case "$name" in
    darwin-arm64)
      local path="$PRESET_ROOT/darwin-arm64"
      if [[ -d "$path" ]]; then
        echo "$path"
        return 0
      fi
      ;;
  esac
  return 1
}

resolve_sysroot() {
  if [[ -n "$CROSS_SYSROOT" ]]; then
    echo "$CROSS_SYSROOT"
    return 0
  fi

  if [[ -n "${REML_TOOLCHAIN_HOME:-}" && -d "${REML_TOOLCHAIN_HOME}/sysroot" ]]; then
    echo "${REML_TOOLCHAIN_HOME}/sysroot"
    return 0
  fi

  return 1
}

# ========== メイン ==========

main() {
  if ! LLVM_AS=$(resolve_tool "$LLVM_AS_CANDIDATE" "llvm-as" 19 18 17); then
    error "llvm-as が見つかりません。LLVM 18+ をインストールしてください。"
  fi
  if ! OPT=$(resolve_tool "$OPT_CANDIDATE" "opt" 19 18 17); then
    error "opt が見つかりません。LLVM 18+ をインストールしてください。"
  fi
  if ! LLC=$(resolve_tool "$LLC_CANDIDATE" "llc" 19 18 17); then
    error "llc が見つかりません。LLVM 18+ をインストールしてください。"
  fi

  local input_ll=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --target)
        shift || error "--target の直後にターゲットトリプルを指定してください"
        TARGET_TRIPLE="$1"
        shift
        ;;
      --preset)
        shift || error "--preset の直後にプリセット名を指定してください"
        PRESET_NAME="$1"
        shift
        ;;
      --cross)
        CROSS_MODE=1
        shift
        ;;
      --cross-prefix)
        shift || error "--cross-prefix の直後にターゲットトリプルを指定してください"
        CROSS_PREFIX="$1"
        shift
        ;;
      --sysroot)
        shift || error "--sysroot の直後にパスを指定してください"
        CROSS_SYSROOT="$1"
        shift
        ;;
      --ld)
        shift || error "--ld の直後にパスを指定してください"
        CROSS_LD="$1"
        shift
        ;;
      --objcopy)
        shift || error "--objcopy の直後にパスを指定してください"
        CROSS_OBJCOPY="$1"
        shift
        ;;
      --output)
        shift || error "--output の直後にパスを指定してください"
        CROSS_OUTPUT="$1"
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      --)
        shift
        if [[ $# -gt 0 ]]; then
          input_ll="$1"
          shift
        fi
        break
        ;;
      -*)
        error "不明なオプションです: $1"
        ;;
      *)
        if [[ -z "$input_ll" ]]; then
          input_ll="$1"
        else
          error "入力ファイルは一つだけ指定してください"
        fi
        shift
        ;;
    esac
  done

  if [[ -z "$input_ll" && -n "$PRESET_NAME" ]]; then
    local preset_path
    if ! preset_path=$(resolve_preset "$PRESET_NAME"); then
      error "不明なプリセットです: $PRESET_NAME"
    fi
    input_ll="$preset_path"
  fi

  if [[ -z "$input_ll" ]]; then
    usage >&2
    exit 1
  fi

  if (( CROSS_MODE )); then
    if ! CROSS_SYSROOT=$(resolve_sysroot); then
      error "--sysroot が指定されていないか、REML_TOOLCHAIN_HOME/sysroot が見つかりません。"
    fi
    if [[ ! -d "$CROSS_SYSROOT" ]]; then
      error "sysroot が存在しません: $CROSS_SYSROOT"
    fi
  fi

  check_llvm_version

  if [[ -d "$input_ll" ]]; then
    local files=()
    while IFS= read -r file; do
      files+=("$file")
    done < <(find "$input_ll" -type f -name '*.ll' | sort)
    if [[ ${#files[@]} -eq 0 ]]; then
      error "ディレクトリ内に .ll ファイルが見つかりませんでした: $input_ll"
    fi
    local status=0
    for file in "${files[@]}"; do
      echo ""
      echo ">>> $file の検証を開始"
      if ! verify_single "$file"; then
        status=$?
      fi
    done
    exit "$status"
  fi

  if [[ ! -f "$input_ll" ]]; then
    error "入力ファイルが存在しません: $input_ll"
  fi

  verify_single "$input_ll"
  exit $?
}

verify_single() {
  local input_ll="$1"

  local temp_bc="${input_ll%.ll}.bc"
  local temp_obj="${input_ll%.ll}.o"
  local temp_asm="${input_ll%.ll}.s"
  local temp_exe=""
  local temp_stripped=""

  cleanup() {
    rm -f "$temp_bc" "$temp_obj" "$temp_asm"
  }
  trap cleanup EXIT

  echo "========================================="
  echo "LLVM IR 検証パイプライン"
  echo "========================================="
  echo "入力: $input_ll"
  echo ""

  echo "[1/3] llvm-as: アセンブル (.ll → .bc)..."
  if ! "$LLVM_AS" "$input_ll" -o "$temp_bc" 2>&1; then
    echo "llvm-as が失敗しました（終了コード: 2）" >&2
    trap - EXIT
    return 2
  fi
  echo "✓ llvm-as 成功"

  echo "[2/3] opt -verify: LLVM 検証パス実行..."
  if ! "$OPT" -passes=verify -disable-output "$temp_bc" 2>&1; then
    echo "opt -verify が失敗しました（終了コード: 3）" >&2
    trap - EXIT
    return 3
  fi
  echo "✓ opt -verify 成功"

  echo "[3/3] llc: ネイティブコード生成 (.bc → .o)..."
  local -a llc_cmd=("$LLC" "-filetype=obj")

  # --target が指定されている場合はそれを使用
  if [[ -n "$TARGET_TRIPLE" ]]; then
    llc_cmd+=("-mtriple=$TARGET_TRIPLE")
    echo "ターゲットトリプル: $TARGET_TRIPLE"
  elif (( CROSS_MODE )); then
    llc_cmd+=("-mtriple=$CROSS_PREFIX")
    echo "ターゲットトリプル: $CROSS_PREFIX"
  fi

  if ! "${llc_cmd[@]}" "$temp_bc" -o "$temp_obj" 2>&1; then
    echo "llc が失敗しました（終了コード: 4）" >&2
    trap - EXIT
    return 4
  fi
  echo "✓ llc 成功"

  if (( CROSS_MODE )); then
    echo ""
    echo "========================================="
    echo "クロスリンク (ld.lld)"
    echo "========================================="

    local ld_path
    if ! ld_path=$(resolve_ld); then
      error "クロスリンク用 ld.lld が見つかりません (--ld で明示指定してください)。"
    fi

    local sysroot="$CROSS_SYSROOT"
    local crt1 crti crtn dynamic_linker
    crt1=$(find_first_existing \
      "$sysroot/usr/lib/${CROSS_PREFIX}/Scrt1.o" \
      "$sysroot/usr/lib/x86_64-linux-gnu/Scrt1.o" \
      "$sysroot/usr/lib/${CROSS_PREFIX}/crt1.o" \
      "$sysroot/usr/lib/x86_64-linux-gnu/crt1.o") || error "Scrt1/crt1 が sysroot 内で見つかりません。"
    crti=$(find_first_existing \
      "$sysroot/usr/lib/${CROSS_PREFIX}/crti.o" \
      "$sysroot/usr/lib/x86_64-linux-gnu/crti.o") || error "crti.o が sysroot 内で見つかりません。"
    crtn=$(find_first_existing \
      "$sysroot/usr/lib/${CROSS_PREFIX}/crtn.o" \
      "$sysroot/usr/lib/x86_64-linux-gnu/crtn.o") || error "crtn.o が sysroot 内で見つかりません。"
    dynamic_linker=$(find_first_existing \
      "$sysroot/lib64/ld-linux-x86-64.so.2" \
      "$sysroot/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2") || warn "ld-linux-x86-64.so.2 が見つかりません。"

    if [[ -n "$CROSS_OUTPUT" ]]; then
      temp_exe="$CROSS_OUTPUT"
    else
      temp_exe="${input_ll%.ll}.elf"
    fi

    local -a ld_cmd=("$ld_path" "-o" "$temp_exe" "--sysroot=$sysroot")
    if [[ -n "$dynamic_linker" ]]; then
      ld_cmd+=("-dynamic-linker" "${dynamic_linker#"$sysroot"}")
    fi

    ld_cmd+=("$crt1" "$crti" "$temp_obj")

    local -a lib_dirs=(
      "$sysroot/usr/lib/${CROSS_PREFIX}"
      "$sysroot/lib/${CROSS_PREFIX}"
      "$sysroot/usr/lib/x86_64-linux-gnu"
      "$sysroot/lib/x86_64-linux-gnu"
      "$sysroot/usr/lib"
      "$sysroot/lib"
    )
    for dir in "${lib_dirs[@]}"; do
      if [[ -d "$dir" ]]; then
        ld_cmd+=("-L$dir")
      fi
    done

    ld_cmd+=("-lc" "$crtn")

    printf '    %q' "${ld_cmd[@]}"
    echo
    if ! "${ld_cmd[@]}"; then
      echo "ld.lld が失敗しました（終了コード: 5）" >&2
      trap - EXIT
      return 5
    fi
    echo "✓ ld.lld 成功"

    local objcopy_path
    if objcopy_path=$(resolve_objcopy); then
      temp_stripped="${temp_exe%.*}.stripped.elf"
      local -a objcopy_cmd=("$objcopy_path" "--strip-debug" "$temp_exe" "$temp_stripped")
      printf '    %q' "${objcopy_cmd[@]}"
      echo
      if ! "${objcopy_cmd[@]}"; then
        warn "objcopy に失敗しました（処理を継続します）"
        temp_stripped=""
      else
        echo "✓ objcopy 成功"
      fi
    else
      warn "objcopy が見つからないためストリップ処理をスキップしました。"
    fi
  fi

  echo ""
  echo "========================================="
  echo "検証成功 ✓"
  echo "========================================="
  echo "生成物:"
  echo "  - ビットコード: $temp_bc"
  echo "  - オブジェクトファイル: $temp_obj"
  if (( CROSS_MODE )); then
    echo "  - 実行ファイル: ${temp_exe:-<未生成>}"
    if [[ -n "$temp_stripped" ]]; then
      echo "  - objcopy 出力: $temp_stripped"
    fi
  fi
  echo ""

  trap - EXIT
  return 0
}

main "$@"
