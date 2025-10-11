#!/usr/bin/env bash
# ローカル CI 再現スクリプト
#
# GitHub Actions の bootstrap-linux ワークフローと同じ手順をローカルで実行します。
# これにより、CI 環境と同じ検証をローカルで事前に実施できます。
#
# 使い方:
#   ./scripts/ci-local.sh [オプション]
#
# オプション:
#   --target <TARGET> ターゲットプラットフォーム（linux または macos、デフォルト: 自動検出）
#   --skip-lint       Lint ステップをスキップ
#   --skip-build      Build ステップをスキップ
#   --skip-test       Test ステップをスキップ
#   --skip-llvm       LLVM IR 検証ステップをスキップ
#   --skip-runtime    ランタイムテストをスキップ
#   --verbose         詳細なログを出力
#   -h, --help        このヘルプを表示
#
# 前提条件:
#   - OCaml 5.2.1+ がインストールされていること
#   - LLVM 18+ がインストールされていること
#   - opam が設定されていること
#
# 参考:
#   - .github/workflows/bootstrap-linux.yml
#   - docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md

set -euo pipefail

# ========== 設定 ==========

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# デフォルト設定
TARGET=""
SKIP_LINT=0
SKIP_BUILD=0
SKIP_TEST=0
SKIP_LLVM=0
SKIP_RUNTIME=0
VERBOSE=0
CLI_TARGET_NAME=""
LLVM_TARGET_TRIPLE=""

# 色付き出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ========== ヘルパー関数 ==========

usage() {
  sed -n '1,30p' "$0"
}

log_info() {
  echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
  echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warn() {
  echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $*" >&2
}

check_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    log_error "$1 が見つかりません。インストールしてください。"
    exit 1
  fi
}

# ========== 引数解析 ==========

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      shift || { log_error "--target の後に値を指定してください"; exit 1; }
      TARGET="$1"
      shift
      ;;
    --skip-lint)
      SKIP_LINT=1
      shift
      ;;
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    --skip-test)
      SKIP_TEST=1
      shift
      ;;
    --skip-llvm)
      SKIP_LLVM=1
      shift
      ;;
    --skip-runtime)
      SKIP_RUNTIME=1
      shift
      ;;
    --verbose)
      VERBOSE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      log_error "不明なオプション: $1"
      usage >&2
      exit 1
      ;;
  esac
done

# ========== プラットフォーム検出 ==========

if [[ -z "$TARGET" ]]; then
  case "$(uname -s)" in
    Linux*)
      TARGET="linux"
      ;;
    Darwin*)
      TARGET="macos"
      ;;
    *)
      log_error "サポートされていないプラットフォーム: $(uname -s)"
      log_error "--target オプションで明示的に指定してください（linux または macos）"
      exit 1
      ;;
  esac
fi

log_info "ターゲットプラットフォーム: $TARGET"

case "$TARGET" in
  linux)
    CLI_TARGET_NAME="x86_64-linux"
    LLVM_TARGET_TRIPLE="x86_64-unknown-linux-gnu"
    ;;
  macos)
    CLI_TARGET_NAME="x86_64-apple-darwin"
    LLVM_TARGET_TRIPLE="x86_64-apple-darwin"
    ;;
  *)
    CLI_TARGET_NAME=""
    LLVM_TARGET_TRIPLE=""
    ;;
esac

if [[ -n "$CLI_TARGET_NAME" ]]; then
  log_info "コンパイラターゲット: $CLI_TARGET_NAME"
fi
if [[ -n "$LLVM_TARGET_TRIPLE" ]]; then
  log_info "LLVM ターゲットトリプル: $LLVM_TARGET_TRIPLE"
fi

# ========== 環境チェック ==========

log_info "環境チェック中..."

check_command opam
check_command dune

# プラットフォーム固有の LLVM パス設定
if [[ "$TARGET" == "macos" ]]; then
  # macOS: Homebrew でインストールされた LLVM を使用
  if [ -d "/usr/local/opt/llvm@18/bin" ]; then
    export PATH="/usr/local/opt/llvm@18/bin:$PATH"
    log_info "LLVM パスを設定: /usr/local/opt/llvm@18/bin"
  else
    log_warn "Homebrew LLVM パスが見つかりません。tooling/ci/macos/setup-env.sh を実行してください。"
  fi
fi

check_command llvm-as
check_command opt
check_command llc

# OCaml バージョンチェック
OCAML_VERSION=$(opam exec -- ocaml -version 2>&1 | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -1)
log_info "OCaml バージョン: $OCAML_VERSION"

# LLVM バージョンチェック
LLVM_VERSION=$(llvm-as --version 2>&1 | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -1)
log_info "LLVM バージョン: $LLVM_VERSION"

# プラットフォーム固有の依存関係チェック
if [[ "$TARGET" == "macos" ]]; then
  # Homebrew チェック
  if ! command -v brew >/dev/null 2>&1; then
    log_warn "Homebrew が見つかりません。tooling/ci/macos/setup-env.sh でセットアップしてください。"
  fi

  # Xcode Command Line Tools チェック
  if ! xcode-select -p >/dev/null 2>&1; then
    log_error "Xcode Command Line Tools がインストールされていません。"
    log_error "インストール: xcode-select --install"
    exit 1
  fi
fi

# ========== Lint ステップ ==========

if (( ! SKIP_LINT )); then
  log_info "========================================="
  log_info "Lint ステップ (1/5)"
  log_info "========================================="

  cd "$REPO_ROOT/compiler/ocaml"

  log_info "依存関係をインストール中..."
  opam install . --deps-only --with-test --yes

  log_info "コードフォーマットをチェック中..."
  if ! opam exec -- dune build @fmt; then
    log_error "フォーマットチェックに失敗しました。'dune build @fmt --auto-promote' を実行してください。"
    exit 1
  fi

  log_success "Lint ステップ完了"
else
  log_warn "Lint ステップをスキップしました"
fi

# ========== Build ステップ ==========

if (( ! SKIP_BUILD )); then
  log_info "========================================="
  log_info "Build ステップ (2/5)"
  log_info "========================================="

  cd "$REPO_ROOT/compiler/ocaml"

  log_info "コンパイラをビルド中..."
  opam exec -- dune build

  log_success "コンパイラビルド完了"

  cd "$REPO_ROOT/runtime/native"

  log_info "ランタイムライブラリをビルド中..."
  make runtime

  log_success "ランタイムビルド完了"
  log_success "Build ステップ完了"
else
  log_warn "Build ステップをスキップしました"
fi

# ========== Test ステップ ==========

if (( ! SKIP_TEST )); then
  log_info "========================================="
  log_info "Test ステップ (3/5)"
  log_info "========================================="

  cd "$REPO_ROOT/compiler/ocaml"

  log_info "コンパイラテストを実行中..."
  opam exec -- dune runtest

  log_success "コンパイラテスト完了"

  if (( ! SKIP_RUNTIME )); then
    cd "$REPO_ROOT/runtime/native"

    log_info "ランタイムテストを実行中..."
    make test

    log_success "ランタイムテスト完了"

    # Valgrind チェック（Linux のみ、利用可能な場合）
    if [[ "$TARGET" == "linux" ]]; then
      if command -v valgrind >/dev/null 2>&1; then
        log_info "Valgrind メモリチェックを実行中..."
        for test in build/test_*; do
          if [ -x "$test" ]; then
            log_info "  Checking $(basename "$test")..."
            valgrind --leak-check=full --error-exitcode=1 --suppressions=/dev/null "$test" || exit 1
          fi
        done
        log_success "Valgrind メモリチェック完了"
      else
        log_warn "Valgrind が見つかりません。メモリチェックをスキップしました。"
      fi
    else
      log_info "macOS では Valgrind をスキップします（AddressSanitizer を使用）"
    fi

    # AddressSanitizer チェック
    log_info "AddressSanitizer チェックを実行中..."
    make clean
    DEBUG=1 make runtime
    DEBUG=1 make test
    log_success "AddressSanitizer チェック完了"
  else
    log_warn "ランタイムテストをスキップしました"
  fi

  log_success "Test ステップ完了"
else
  log_warn "Test ステップをスキップしました"
fi

# ========== LLVM IR 検証ステップ ==========

if (( ! SKIP_LLVM )); then
  log_info "========================================="
  log_info "LLVM IR 検証ステップ (4/5)"
  log_info "========================================="

  cd "$REPO_ROOT/compiler/ocaml"

  log_info "LLVM IR を生成中..."
  LLVM_IR_DIR="/tmp/reml-ci-local-llvm-ir-$$"
  mkdir -p "$LLVM_IR_DIR"

  for example in examples/cli/*.reml; do
    if [ -f "$example" ]; then
      log_info "  Generating IR for $(basename "$example")..."
      if [[ -n "$CLI_TARGET_NAME" ]]; then
        opam exec -- dune exec -- remlc "$example" --target "$CLI_TARGET_NAME" --emit-ir --out-dir="$LLVM_IR_DIR" || true
      else
        opam exec -- dune exec -- remlc "$example" --emit-ir --out-dir="$LLVM_IR_DIR" || true
      fi
    fi
  done

  log_info "生成された LLVM IR を検証中..."
  chmod +x "$REPO_ROOT/compiler/ocaml/scripts/verify_llvm_ir.sh"

  for ir_file in "$LLVM_IR_DIR"/*.ll; do
    if [ -f "$ir_file" ]; then
      log_info "  Verifying $(basename "$ir_file")..."
      if [[ -n "$LLVM_TARGET_TRIPLE" ]]; then
        "$REPO_ROOT/compiler/ocaml/scripts/verify_llvm_ir.sh" --target "$LLVM_TARGET_TRIPLE" "$ir_file" || exit 1
      else
        "$REPO_ROOT/compiler/ocaml/scripts/verify_llvm_ir.sh" "$ir_file" || exit 1
      fi
    fi
  done

  log_success "LLVM IR 検証完了"
  log_info "生成された LLVM IR: $LLVM_IR_DIR"
else
  log_warn "LLVM IR 検証ステップをスキップしました"
fi

# ========== 完了 ==========

log_info "========================================="
log_success "すべての CI ステップが完了しました ✓"
log_info "========================================="
log_info ""
log_info "次のステップ:"
log_info "  - コミット前に 'git status' で変更を確認"
log_info "  - GitHub Actions で同じテストが実行されます"
log_info ""

exit 0
