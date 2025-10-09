#!/usr/bin/env bash
# クロスツールチェーンを用いて Reml サンプルを Linux x86_64 バイナリへビルドするスクリプト
# docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §10 の自動化。

set -euo pipefail

usage() {
  cat <<'USAGE'
使用方法: scripts/toolchain/build-linux-sample.sh [オプション]

主要オプション:
      --source <PATH>        コンパイルする Reml ソース（既定: compiler/ocaml/tests/integration/test_runtime_link.reml）
      --output <PATH>        生成する ELF バイナリの出力先（既定: tooling/toolchains/examples/hello-linux）
      --target <TRIPLE>      クロスターゲットトリプル（既定: x86_64-unknown-linux-gnu）
      --sysroot <PATH>       明示的な sysroot（既定: REML_TOOLCHAIN_HOME/sysroot）
      --build-runtime        ランタイム (runtime/native) を CROSS=1 で再ビルド
      --help                 ヘルプを表示

環境変数:
  REML_TOOLCHAIN_HOME        クロスツールチェーンホーム（未指定時は tooling/toolchains/x86_64-unknown-linux-gnu）
  LLVM_PREFIX_OVERRIDE 等は scripts/toolchain/prepare-linux_x86_64.sh に準ずる
USAGE
}

log()  { echo "[info] $*"; }
warn() { echo "[warn] $*" >&2; }
die()  { echo "[error] $*" >&2; exit 1; }

repo_root() {
  local script_dir
  script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
  cd "${script_dir}/../.." && pwd
}

main() {
  local repo
  repo=$(repo_root)

  local source_path="${repo}/compiler/ocaml/tests/integration/test_runtime_link.reml"
  local output_path="${repo}/tooling/toolchains/examples/hello-linux"
  local target_triple="x86_64-unknown-linux-gnu"
  local sysroot_override=""
  local rebuild_runtime=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --source)
        shift || die "--source の直後にパスを指定してください"
        source_path="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
        ;;
      --output)
        shift || die "--output の直後にパスを指定してください"
        output_path="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
        ;;
      --target)
        shift || die "--target の直後にターゲットトリプルを指定してください"
        target_triple="$1"
        ;;
      --sysroot)
        shift || die "--sysroot の直後にパスを指定してください"
        sysroot_override="$(cd "$1" && pwd)"
        ;;
      --build-runtime)
        rebuild_runtime=1
        ;;
      --help|-h)
        usage
        exit 0
        ;;
      *)
        die "不明なオプションです: $1"
        ;;
    esac
    shift
  done

  [[ -f "$source_path" ]] || die "指定されたソースファイルが存在しません: $source_path"

  local toolchain_home="${REML_TOOLCHAIN_HOME:-${repo}/tooling/toolchains/x86_64-unknown-linux-gnu}"
  local env_file="${toolchain_home}/env.sh"
  [[ -f "$env_file" ]] || die "クロスツールチェーンenvが見つかりません: $env_file. prepare スクリプトを実行してください。"

  # shellcheck disable=SC1090
  source "$env_file"

  local sysroot="${sysroot_override:-${toolchain_home}/sysroot}"
  [[ -d "$sysroot" ]] || die "sysroot ディレクトリが存在しません: $sysroot"

  local ld_path
  ld_path=$(command -v ld.lld 2>/dev/null) || die "ld.lld が見つかりません。LLVM ツールチェーンを確認してください。"
  local objcopy_path
  objcopy_path=$(command -v llvm-objcopy 2>/dev/null) || die "llvm-objcopy が見つかりません。LLVM ツールチェーンを確認してください。"

  log "Reml ソース: $source_path"
  log "出力バイナリ: $output_path"
  log "ツールチェーン: $toolchain_home"
  log "sysroot: $sysroot"
  log "ld.lld: $ld_path"
  log "llvm-objcopy: $objcopy_path"

  # 1. OCaml コンパイラをビルド
  log "dune build (OCaml コンパイラ)"
  (
    cd "${repo}/compiler/ocaml"
    opam exec -- dune build
  )

  # 2. ランタイムをクロスビルド
  if (( rebuild_runtime )); then
    log "runtime/native をクロスビルド (CROSS=1)"
    if ! make -C "${repo}/runtime/native" runtime \
        CROSS=1 \
        CROSS_PREFIX="$target_triple" \
        CROSS_SYSROOT="$sysroot"; then
      warn "ランタイムのクロスビルドに失敗しました（sysroot にヘッダが不足している可能性があります）。"
      warn "必要に応じて --build-runtime を外し、docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §10.2 を参照してください。"
    fi
  else
    log "ランタイムの再ビルドをスキップ (--build-runtime 未指定)"
  fi

  # 3. Reml ソースから LLVM IR を生成
  local temp_dir
  temp_dir=$(mktemp -d)
  trap 'tmp=${temp_dir:-}; if [[ -n "$tmp" && -d "$tmp" ]]; then rm -rf "$tmp"; fi' EXIT

  local source_dir
  source_dir=$(dirname "$source_path")
  local source_file
  source_file=$(basename "$source_path")
  local source_base="${source_file%.reml}"

  log "remlc --emit-ir を実行"
  (
    cd "$source_dir"
    opam exec -- dune exec --root "${repo}/compiler/ocaml" -- remlc \
      --emit-ir \
      --out-dir "$temp_dir" \
      --target x86_64-linux \
      "$source_path"
  )

  local ll_file="${temp_dir}/${source_base}.ll"
  [[ -f "$ll_file" ]] || die "LLVM IR が生成されませんでした: $ll_file"

  # 4. クロスリンクして ELF を生成
  local temp_output="${temp_dir}/${source_base}.elf"
  log "verify_llvm_ir.sh --cross で ELF を生成"
  compiler/ocaml/scripts/verify_llvm_ir.sh \
    --cross \
    --cross-prefix "$target_triple" \
    --sysroot "$sysroot" \
    --ld "$ld_path" \
    --objcopy "$objcopy_path" \
    --output "$temp_output" \
    "$ll_file"

  local stripped_candidate="${temp_output%.elf}.stripped.elf"
  local final_bin="$temp_output"
  if [[ -f "$stripped_candidate" ]]; then
    final_bin="$stripped_candidate"
  fi

  local examples_dir
  examples_dir=$(dirname "$output_path")
  mkdir -p "$examples_dir"

  cp "$ll_file" "${output_path}.ll"
  cp "$final_bin" "$output_path"
  chmod +x "$output_path"

  log "生成したファイル:"
  log "  - LLVM IR: ${output_path}.ll"
  log "  - ELF: $output_path"
  log "完了しました。"
}

main "$@"
