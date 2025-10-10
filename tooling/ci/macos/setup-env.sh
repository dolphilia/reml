#!/usr/bin/env bash
# macOS 開発環境セットアップスクリプト
#
# GitHub Actions macOS ランナーおよびローカル macOS 環境で
# Reml コンパイラのビルドに必要な依存関係をセットアップします。
#
# 使い方:
#   ./tooling/ci/macos/setup-env.sh [オプション]
#
# オプション:
#   --skip-llvm       LLVM インストールをスキップ
#   --skip-opam       opam セットアップをスキップ
#   --llvm-version    LLVM バージョン（デフォルト: 15）
#   --ocaml-version   OCaml バージョン（デフォルト: 5.2.1）
#   --dry-run         実際にはインストールせず、コマンドのみ表示
#   -h, --help        このヘルプを表示
#
# 前提条件:
#   - macOS 10.15 以降
#   - Homebrew がインストールされていること
#   - Xcode Command Line Tools がインストールされていること
#
# 参考:
#   - docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md
#   - docs/plans/bootstrap-roadmap/1-7-to-1-8-handover.md

set -euo pipefail

# ========== 設定 ==========

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# デフォルト設定
SKIP_LLVM=0
SKIP_OPAM=0
LLVM_VERSION=18
OCAML_VERSION=5.2.1
DRY_RUN=0

# 色付き出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ========== ヘルパー関数 ==========

usage() {
  sed -n '1,24p' "$0"
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

run_command() {
  if (( DRY_RUN )); then
    echo "[DRY-RUN] $*"
  else
    "$@"
  fi
}

check_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    return 1
  fi
  return 0
}

# ========== 引数解析 ==========

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-llvm)
      SKIP_LLVM=1
      shift
      ;;
    --skip-opam)
      SKIP_OPAM=1
      shift
      ;;
    --llvm-version)
      shift || { log_error "--llvm-version の後に値を指定してください"; exit 1; }
      LLVM_VERSION="$1"
      shift
      ;;
    --ocaml-version)
      shift || { log_error "--ocaml-version の後に値を指定してください"; exit 1; }
      OCAML_VERSION="$1"
      shift
      ;;
    --dry-run)
      DRY_RUN=1
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

# ========== 環境チェック ==========

log_info "========================================="
log_info "macOS 開発環境セットアップ"
log_info "========================================="
log_info ""

# macOS バージョンチェック
MACOS_VERSION=$(sw_vers -productVersion)
log_info "macOS バージョン: $MACOS_VERSION"

# Homebrew チェック
if ! check_command brew; then
  log_error "Homebrew がインストールされていません。"
  log_error "以下のコマンドでインストールしてください:"
  log_error '  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'
  exit 1
fi

BREW_VERSION=$(brew --version | head -1)
log_info "Homebrew: $BREW_VERSION"

# Xcode Command Line Tools チェック
if ! xcode-select -p >/dev/null 2>&1; then
  log_error "Xcode Command Line Tools がインストールされていません。"
  log_error "以下のコマンドでインストールしてください:"
  log_error "  xcode-select --install"
  exit 1
fi

XCODE_PATH=$(xcode-select -p)
log_info "Xcode Command Line Tools: $XCODE_PATH"

# clang バージョンチェック
CLANG_VERSION=$(clang --version | head -1)
log_info "clang: $CLANG_VERSION"

log_info ""

# ========== LLVM セットアップ ==========

if (( ! SKIP_LLVM )); then
  log_info "========================================="
  log_info "LLVM $LLVM_VERSION のセットアップ"
  log_info "========================================="

  # 既にインストールされているかチェック
  if brew list llvm@$LLVM_VERSION >/dev/null 2>&1; then
    log_info "llvm@$LLVM_VERSION は既にインストールされています"
  else
    log_info "llvm@$LLVM_VERSION をインストール中..."
    run_command brew install llvm@$LLVM_VERSION
  fi

  # LLVM のリンク設定
  log_info "llvm@$LLVM_VERSION をリンク中..."
  run_command brew link --force llvm@$LLVM_VERSION || true

  # LLVM パス設定
  LLVM_PATH="/usr/local/opt/llvm@$LLVM_VERSION/bin"

  if (( ! DRY_RUN )); then
    if [ -d "$LLVM_PATH" ]; then
      log_info "LLVM パス: $LLVM_PATH"

      # パス設定を .zshrc または .bash_profile に追加
      SHELL_RC=""
      if [ -n "${SHELL:-}" ]; then
        case "$SHELL" in
          */zsh)
            SHELL_RC="$HOME/.zshrc"
            ;;
          */bash)
            SHELL_RC="$HOME/.bash_profile"
            ;;
        esac
      fi

      if [ -n "$SHELL_RC" ]; then
        if ! grep -q "llvm@$LLVM_VERSION/bin" "$SHELL_RC" 2>/dev/null; then
          log_info "LLVM パスを $SHELL_RC に追加中..."
          echo "" >> "$SHELL_RC"
          echo "# Reml コンパイラ: LLVM $LLVM_VERSION" >> "$SHELL_RC"
          echo "export PATH=\"$LLVM_PATH:\$PATH\"" >> "$SHELL_RC"
          log_success "パス設定を追加しました。シェルを再起動するか 'source $SHELL_RC' を実行してください。"
        fi
      fi

      # 現在のセッションでパスを設定
      export PATH="$LLVM_PATH:$PATH"
    else
      log_warn "LLVM パスが見つかりません: $LLVM_PATH"
    fi

    # LLVM バージョン確認
    if check_command llvm-as; then
      LLVM_AS_VERSION=$(llvm-as --version 2>&1 | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -1 || echo "unknown")
      log_info "llvm-as バージョン: $LLVM_AS_VERSION"
    fi

    if check_command opt; then
      OPT_VERSION=$(opt --version 2>&1 | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -1 || echo "unknown")
      log_info "opt バージョン: $OPT_VERSION"
    fi

    if check_command llc; then
      LLC_VERSION=$(llc --version 2>&1 | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -1 || echo "unknown")
      log_info "llc バージョン: $LLC_VERSION"
    fi
  fi

  log_success "LLVM セットアップ完了"
  log_info ""
else
  log_warn "LLVM セットアップをスキップしました"
  log_info ""
fi

# ========== 追加ツールのセットアップ ==========

log_info "========================================="
log_info "追加ツールのセットアップ"
log_info "========================================="

REQUIRED_TOOLS=(pkg-config libtool)

for tool in "${REQUIRED_TOOLS[@]}"; do
  if brew list "$tool" >/dev/null 2>&1; then
    log_info "$tool は既にインストールされています"
  else
    log_info "$tool をインストール中..."
    run_command brew install "$tool"
  fi
done

log_success "追加ツールのセットアップ完了"
log_info ""

# ========== opam セットアップ ==========

if (( ! SKIP_OPAM )); then
  log_info "========================================="
  log_info "OCaml / opam のセットアップ"
  log_info "========================================="

  # opam のインストール確認
  if ! check_command opam; then
    log_info "opam をインストール中..."
    run_command brew install opam
  else
    log_info "opam は既にインストールされています"
  fi

  if (( ! DRY_RUN )); then
    OPAM_VERSION=$(opam --version 2>&1 || echo "unknown")
    log_info "opam バージョン: $OPAM_VERSION"

    # opam の初期化（未初期化の場合のみ）
    if [ ! -d "$HOME/.opam" ]; then
      log_info "opam を初期化中..."
      run_command opam init --auto-setup --yes
    fi

    # OCaml スイッチの作成または切り替え
    CURRENT_SWITCH=$(opam switch show 2>/dev/null || echo "")

    if [ "$CURRENT_SWITCH" != "$OCAML_VERSION" ]; then
      if opam switch list 2>/dev/null | grep -q "^$OCAML_VERSION "; then
        log_info "OCaml $OCAML_VERSION スイッチに切り替え中..."
        run_command opam switch "$OCAML_VERSION"
      else
        log_info "OCaml $OCAML_VERSION スイッチを作成中..."
        run_command opam switch create "$OCAML_VERSION" --yes
      fi
    else
      log_info "既に OCaml $OCAML_VERSION スイッチを使用中です"
    fi

    # opam 環境変数の設定
    log_info "opam 環境変数を設定中..."
    eval "$(opam env)"

    # OCaml バージョン確認
    if check_command ocaml; then
      OCAML_INSTALLED_VERSION=$(ocaml -version 2>&1 | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -1 || echo "unknown")
      log_info "OCaml バージョン: $OCAML_INSTALLED_VERSION"
    fi

    # dune のインストール確認
    if ! check_command dune; then
      log_info "dune をインストール中..."
      run_command opam install dune --yes
    else
      log_info "dune は既にインストールされています"
      DUNE_VERSION=$(dune --version 2>&1 || echo "unknown")
      log_info "dune バージョン: $DUNE_VERSION"
    fi
  fi

  log_success "opam セットアップ完了"
  log_info ""
else
  log_warn "opam セットアップをスキップしました"
  log_info ""
fi

# ========== 完了 ==========

log_info "========================================="
log_success "macOS 開発環境セットアップが完了しました ✓"
log_info "========================================="
log_info ""
log_info "次のステップ:"
log_info "  1. シェルを再起動するか、以下を実行してパスを有効化:"
log_info "     eval \"\$(opam env)\""
log_info "     export PATH=\"/usr/local/opt/llvm@$LLVM_VERSION/bin:\$PATH\""
log_info ""
log_info "  2. コンパイラをビルド:"
log_info "     cd $REPO_ROOT/compiler/ocaml"
log_info "     opam install . --deps-only --with-test --yes"
log_info "     opam exec -- dune build"
log_info ""
log_info "  3. ランタイムをビルド:"
log_info "     cd $REPO_ROOT/runtime/native"
log_info "     make runtime"
log_info ""
log_info "  4. テストを実行:"
log_info "     cd $REPO_ROOT/compiler/ocaml"
log_info "     opam exec -- dune runtest"
log_info ""
log_info "詳細は compiler/ocaml/README.md を参照してください。"
log_info ""

exit 0
