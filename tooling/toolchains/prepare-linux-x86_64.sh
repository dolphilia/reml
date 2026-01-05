#!/usr/bin/env bash
# macOS → Linux x86_64 クロスコンパイル用ツールチェーン準備スクリプト
# docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §10.2 に対応。

set -euo pipefail

usage() {
  cat <<'USAGE'
使用方法: tooling/toolchains/prepare-linux-x86_64.sh [オプション]

主要オプション:
      --brew                Homebrew から LLVM/LLD などを準備（デフォルト有効）
      --no-brew             Homebrew のセットアップをスキップ
      --archive <PATH>      指定した sysroot アーカイブを展開（.tar, .tar.gz, .tar.zst を想定）
      --cache               既定キャッシュ (tooling/toolchains/cache/debian-bookworm-x86_64.tar.zst) を利用
      --cache-path <PATH>   明示的にキャッシュアーカイブを指定
      --stamp <PATH>        スタンプファイルの保存先（デフォルト: tooling/toolchains/x86_64-unknown-linux-gnu/.stamp-prepared）
      --force               既存スタンプを無視して再構築
      --dry-run             実際には変更せず、予定されている処理のみ表示
  -h, --help                このヘルプを表示

処理の流れ（既定）:
  1. Homebrew で llvm@18 / lld / binutils などを確認・インストール
  2. tooling/toolchains/cache/debian-bookworm-x86_64.tar.zst を sysroot に展開
  3. x86_64-unknown-linux-gnu-* ラッパと env.sh を生成し、スタンプを更新

環境変数:
  REML_TOOLCHAIN_SKIP_WRAPPERS=1   ラッパ生成をスキップ
  REML_TOOLCHAIN_TARGET            ターゲットトリプル（デフォルト: x86_64-unknown-linux-gnu）
  REML_TOOLCHAIN_SYSROOT_SUBDIR    sysroot サブディレクトリ（デフォルト: sysroot）
USAGE
}

log_info()  { echo "[info] $*"; }
log_warn()  { echo "[warn] $*" >&2; }
log_error() { echo "[error] $*" >&2; }

fail() {
  log_error "$@"
  exit 1
}

ensure_command() {
  local cmd="$1"
  local hint="${2:-}"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    if [[ -n "$hint" ]]; then
      fail "${cmd} が見つかりません。${hint}"
    else
      fail "${cmd} が見つかりません。"
    fi
  fi
}

repo_root() {
  local script_dir
  script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
  cd "${script_dir}/../.." && pwd
}

write_wrapper() {
  local dest="$1"
  local tool="$2"
  local preset_args="$3"

  cat >"$dest" <<EOF
#!/usr/bin/env bash
set -euo pipefail

toolchain_home="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")/.." && pwd)"
target_triple="${target_triple}"
sysroot_dir="\${toolchain_home}/${sysroot_subdir}"

exec "${tool}" ${preset_args} "\$@"
EOF
  chmod +x "$dest"
}

write_env_file() {
  local env_path="$1"
  cat >"$env_path" <<EOF
# shellcheck shell=bash
# このファイルは scripts/toolchain/prepare-linux-x86_64.sh によって生成されました。

export REML_TOOLCHAIN_HOME="\${REML_TOOLCHAIN_HOME:-${toolchain_home}}"
export PATH="\${REML_TOOLCHAIN_HOME}/bin:\$PATH"
export QEMU_LD_PREFIX="\${REML_TOOLCHAIN_HOME}/${sysroot_subdir}"
export LD_LIBRARY_PATH="\${REML_TOOLCHAIN_HOME}/${sysroot_subdir}/lib:\${REML_TOOLCHAIN_HOME}/${sysroot_subdir}/lib64:\${LD_LIBRARY_PATH:-}"
EOF
}

write_stamp_file() {
  local stamp="$1"
  cat >"$stamp" <<EOF
prepared_at=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
brew=${brew_enabled}
sysroot_source=${sysroot_source}
stamp_version=1
EOF
}

prepare_brew_packages() {
  log_info "Homebrew パッケージの確認を開始します"
  ensure_command brew "Homebrew をインストールしてください。"

  local -a packages=("llvm@18" "lld" "binutils" "gnu-tar" "coreutils" "pkg-config" "zstd")
  for pkg in "${packages[@]}"; do
    if brew list --versions "$pkg" >/dev/null 2>&1; then
      log_info "パッケージ確認済み: ${pkg}"
    else
      log_info "brew install ${pkg}"
      brew install "$pkg"
    fi
  done

  llvm_prefix=$(brew --prefix llvm@18 2>/dev/null || true)
  if [[ -z "${llvm_prefix}" ]]; then
    llvm_prefix=$(brew --prefix llvm 2>/dev/null || true)
  fi
  [[ -n "${llvm_prefix}" ]] || fail "llvm@18 が見つかりません。brew install llvm@18 を実行してください。"

  lld_prefix=$(brew --prefix lld 2>/dev/null || true)
  if [[ -z "${lld_prefix}" ]]; then
    lld_prefix="${llvm_prefix}"
  fi

  binutils_prefix=$(brew --prefix binutils 2>/dev/null || true)
  if [[ -z "${binutils_prefix}" ]]; then
    log_warn "binutils が見つからないため LLVM binutils を使用します"
    binutils_prefix="${llvm_prefix}"
  fi

  log_info "LLVM prefix: ${llvm_prefix}"
  log_info "LLD prefix: ${lld_prefix}"
  log_info "binutils prefix: ${binutils_prefix}"
}

prepare_sysroot_from_archive() {
  local archive_path="$1"
  [[ -f "$archive_path" ]] || fail "sysroot アーカイブが見つかりません: ${archive_path}"

  ensure_command tar "GNU tar (gnu-tar) をインストールしてください。"

  local dest="${toolchain_home}/${sysroot_subdir}"
  mkdir -p "$dest"

  log_info "sysroot を展開します: ${archive_path}"
  rm -rf "${dest:?}/"*

  local -a tar_opts=("--exclude=dev/*")

  case "$archive_path" in
    *.tar.zst)
      if tar --help 2>&1 | grep -q -- "--zstd"; then
        tar --zstd "${tar_opts[@]}" -xf "$archive_path" -C "$dest"
      else
        ensure_command zstd "brew install zstd を実行してください。"
        zstd -dc "$archive_path" | tar "${tar_opts[@]}" -xf - -C "$dest"
      fi
      ;;
    *.tar.gz|*.tgz)
      tar "${tar_opts[@]}" -xzf "$archive_path" -C "$dest"
      ;;
    *.tar)
      tar "${tar_opts[@]}" -xf "$archive_path" -C "$dest"
      ;;
    *)
      fail "未対応のアーカイブ形式です: ${archive_path}"
      ;;
  esac

  log_info "sysroot 展開完了: ${dest}"
}

generate_wrappers() {
  if [[ "${REML_TOOLCHAIN_SKIP_WRAPPERS:-0}" == "1" ]]; then
    log_warn "ラッパ生成をスキップします (REML_TOOLCHAIN_SKIP_WRAPPERS=1)"
    return
  fi

  mkdir -p "${toolchain_home}/bin"

  local clang_path="${llvm_prefix}/bin/clang"
  local clangxx_path="${llvm_prefix}/bin/clang++"
  local lld_path="${lld_prefix}/bin/ld.lld"

  [[ -x "$clang_path" ]] || fail "clang が見つかりません (${clang_path})"
  [[ -x "$clangxx_path" ]] || fail "clang++ が見つかりません (${clangxx_path})"
  [[ -x "$lld_path" ]] || fail "ld.lld が見つかりません (${lld_path})"

  log_info "ラッパスクリプトを生成します (${toolchain_home}/bin)"

  write_wrapper "${toolchain_home}/bin/${target_triple}-clang" "\"${clang_path}\"" "--target=\"\${target_triple}\" --sysroot=\"\${sysroot_dir}\""
  write_wrapper "${toolchain_home}/bin/${target_triple}-clang++" "\"${clangxx_path}\"" "--target=\"\${target_triple}\" --sysroot=\"\${sysroot_dir}\""
  write_wrapper "${toolchain_home}/bin/${target_triple}-ld" "\"${lld_path}\"" "--sysroot=\"\${sysroot_dir}\""

  if [[ -n "${binutils_prefix}" ]]; then
    local -A binutils_map=(
      ["ar"]="${binutils_prefix}/bin/${target_triple}-ar"
      ["ranlib"]="${binutils_prefix}/bin/${target_triple}-ranlib"
      ["objcopy"]="${binutils_prefix}/bin/${target_triple}-objcopy"
      ["objdump"]="${binutils_prefix}/bin/${target_triple}-objdump"
      ["strip"]="${binutils_prefix}/bin/${target_triple}-strip"
      ["readelf"]="${binutils_prefix}/bin/${target_triple}-readelf"
    )

    for tool in "${!binutils_map[@]}"; do
      local candidate="${binutils_map[$tool]}"
      if [[ -x "$candidate" ]]; then
        ln -sf "$candidate" "${toolchain_home}/bin/${target_triple}-${tool}"
        log_info "リンク作成: ${toolchain_home}/bin/${target_triple}-${tool}"
      else
        log_warn "${candidate} が見つかりません。${tool} は LLVM 版で代替してください。"
      fi
    done
  else
    log_warn "binutils プレフィックスが設定されていないため、binutils のラッパ生成をスキップします。"
  fi
}

main() {
  local repo
  repo=$(repo_root)
  local default_cache="${repo}/tooling/toolchains/cache/debian-bookworm-x86_64.tar.zst"

  toolchain_home="${repo}/tooling/toolchains/x86_64-unknown-linux-gnu"
  sysroot_subdir="${REML_TOOLCHAIN_SYSROOT_SUBDIR:-sysroot}"
  target_triple="${REML_TOOLCHAIN_TARGET:-x86_64-unknown-linux-gnu}"
  stamp_file="${toolchain_home}/.stamp-prepared"

  local force=0
  local dry_run=0
  local brew_enabled=1
  sysroot_source="cache"
  local sysroot_archive="$default_cache"

  local llvm_prefix="${LLVM_PREFIX_OVERRIDE:-}"
  local lld_prefix="${LLD_PREFIX_OVERRIDE:-}"
  local binutils_prefix="${BINUTILS_PREFIX_OVERRIDE:-}"

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --brew)
        brew_enabled=1
        ;;
      --no-brew)
        brew_enabled=0
        ;;
      --archive)
        shift || fail "--archive の直後にパスを指定してください"
        [[ -n "${1:-}" ]] || fail "--archive の値が空です"
        if [[ "$sysroot_source" != "cache" ]]; then
          fail "--archive と --cache/--cache-path は同時に指定できません"
        fi
        sysroot_source="archive"
        sysroot_archive="$1"
        ;;
      --cache)
        if [[ "$sysroot_source" == "archive" ]]; then
          fail "--archive と --cache は同時に指定できません"
        fi
        sysroot_source="cache"
        sysroot_archive="$default_cache"
        ;;
      --cache-path)
        shift || fail "--cache-path の直後にパスを指定してください"
        [[ -n "${1:-}" ]] || fail "--cache-path の値が空です"
        if [[ "$sysroot_source" == "archive" ]]; then
          fail "--archive と --cache-path は同時に指定できません"
        fi
        sysroot_source="cache"
        sysroot_archive="$1"
        ;;
      --stamp)
        shift || fail "--stamp の直後にパスを指定してください"
        stamp_file="$1"
        ;;
      --force)
        force=1
        ;;
      --dry-run)
        dry_run=1
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        fail "不明な引数です: $1"
        ;;
    esac
    shift
  done

  mkdir -p "${toolchain_home}"

  if [[ -f "$stamp_file" && $force -eq 0 ]]; then
    log_info "スタンプが存在します (${stamp_file})。--force なしで再実行すると処理をスキップします。"
    log_info "既存の環境を使用します。"
    exit 0
  fi

  log_info "レポジトリルート: ${repo}"
  log_info "ツールチェーンホーム: ${toolchain_home}"
  log_info "ターゲットトリプル: ${target_triple}"
  log_info "sysroot ソース: ${sysroot_source}"
  log_info "sysroot アーカイブ: ${sysroot_archive}"

  if [[ $dry_run -eq 1 ]]; then
    log_info "dry-run モードのため、設定状況のみ出力します。"
    if [[ $brew_enabled -eq 1 ]]; then
      log_info "[dry-run] Homebrew パッケージを確認します。"
    else
      log_info "[dry-run] Homebrew セットアップはスキップします。"
    fi
    log_info "[dry-run] sysroot アーカイブ: ${sysroot_archive}"
    log_info "[dry-run] env.sh とスタンプ: ${toolchain_home}"
    exit 0
  fi

  if [[ $brew_enabled -eq 1 ]]; then
    prepare_brew_packages
    generate_wrappers
  else
    log_warn "Homebrew セットアップをスキップしました (--no-brew)"
    if [[ -z "${llvm_prefix}" || -z "${lld_prefix}" ]]; then
      fail "LLVM_PREFIX_OVERRIDE と LLD_PREFIX_OVERRIDE を指定してください (--no-brew 利用時)。"
    fi
    generate_wrappers
  fi

  case "$sysroot_source" in
    archive|cache)
      prepare_sysroot_from_archive "$sysroot_archive"
      ;;
    *)
      fail "未知の sysroot_source: ${sysroot_source}"
      ;;
  esac

  write_env_file "${toolchain_home}/env.sh"
  write_stamp_file "$stamp_file"
  log_info "env.sh とスタンプファイルを更新しました。"

  log_info "macOS → Linux x86_64 ツールチェーンの準備が完了しました。"
}

main "$@"
