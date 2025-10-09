#!/usr/bin/env bash
# LLVM IR検証パイプライン（Phase 3 Week 15-16）
#
# このスクリプトは生成されたLLVM IRを3段階で検証する:
# 1. llvm-as: アセンブル (.ll → .bc)
# 2. opt -verify: LLVM検証パス実行
# 3. llc: ネイティブコード生成 (.bc → .o)
#
# 使い方:
#   ./scripts/verify_llvm_ir.sh <input.ll>
#
# 終了コード:
#   0: 検証成功
#   1: 引数エラー
#   2: llvm-as失敗
#   3: opt -verify失敗
#   4: llc失敗
#
# 参考:
# - docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md §6

set -euo pipefail

# ========== 設定 ==========

# LLVM バージョン（最小 15.0）
LLVM_MIN_VERSION="15.0"

# llvm-as, opt, llc のパス（環境変数で上書き可能）
LLVM_AS="${LLVM_AS:-llvm-as}"
OPT="${OPT:-opt}"
LLC="${LLC:-llc}"

# ========== エラーハンドリング ==========

error() {
  echo "エラー: $*" >&2
  exit 1
}

warn() {
  echo "警告: $*" >&2
}

# ========== LLVM バージョン確認 ==========

check_llvm_version() {
  if ! command -v "$LLVM_AS" &> /dev/null; then
    error "llvm-as が見つかりません。LLVM 15+ をインストールしてください。"
  fi

  # バージョン取得（例: "LLVM version 18.1.8"）
  local version_output
  version_output=$("$LLVM_AS" --version 2>&1 | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -1)

  if [[ -z "$version_output" ]]; then
    warn "LLVM バージョンを検出できませんでした。続行します..."
    return 0
  fi

  # バージョン比較（簡易実装: メジャーバージョンのみ）
  local major_version
  major_version=$(echo "$version_output" | cut -d. -f1)

  if (( major_version < 15 )); then
    error "LLVM $version_output が検出されましたが、LLVM 15+ が必要です。"
  fi

  echo "LLVM $version_output を使用します。"
}

# ========== メイン処理 ==========

main() {
  # 引数チェック
  if [[ $# -ne 1 ]]; then
    echo "使い方: $0 <input.ll>" >&2
    exit 1
  fi

  local input_ll="$1"

  # 入力ファイル存在確認
  if [[ ! -f "$input_ll" ]]; then
    error "入力ファイルが存在しません: $input_ll"
  fi

  # LLVM バージョン確認
  check_llvm_version

  # 一時ファイル設定
  local temp_bc="${input_ll%.ll}.bc"
  local temp_obj="${input_ll%.ll}.o"
  local temp_asm="${input_ll%.ll}.s"

  # クリーンアップ関数
  cleanup() {
    rm -f "$temp_bc" "$temp_obj" "$temp_asm"
  }
  trap cleanup EXIT

  echo "========================================="
  echo "LLVM IR 検証パイプライン"
  echo "========================================="
  echo "入力: $input_ll"
  echo ""

  # ステップ1: llvm-as（アセンブル）
  echo "[1/3] llvm-as: アセンブル (.ll → .bc)..."
  if ! "$LLVM_AS" "$input_ll" -o "$temp_bc" 2>&1; then
    echo "llvm-as が失敗しました（終了コード: 2）" >&2
    exit 2
  fi
  echo "✓ llvm-as 成功"

  # ステップ2: opt -verify（検証パス）
  echo "[2/3] opt -verify: LLVM 検証パス実行..."
  if ! "$OPT" -passes=verify -disable-output "$temp_bc" 2>&1; then
    echo "opt -verify が失敗しました（終了コード: 3）" >&2
    exit 3
  fi
  echo "✓ opt -verify 成功"

  # ステップ3: llc（ネイティブコード生成）
  echo "[3/3] llc: ネイティブコード生成 (.bc → .o)..."
  if ! "$LLC" -filetype=obj "$temp_bc" -o "$temp_obj" 2>&1; then
    echo "llc が失敗しました（終了コード: 4）" >&2
    exit 4
  fi
  echo "✓ llc 成功"

  echo ""
  echo "========================================="
  echo "検証成功 ✓"
  echo "========================================="
  echo "生成物:"
  echo "  - ビットコード: $temp_bc"
  echo "  - オブジェクトファイル: $temp_obj"
  echo ""

  exit 0
}

main "$@"
