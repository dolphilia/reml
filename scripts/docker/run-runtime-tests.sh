#!/usr/bin/env bash
# Docker コンテナ内で Phase 1 ランタイム統合テストを実行するユーティリティ

set -euo pipefail

usage() {
  cat <<'USAGE'
使用方法: scripts/docker/run-runtime-tests.sh [-t <tag>] [-- <custom command>]

オプション:
  -t, --tag       実行するイメージタグ（デフォルト: ghcr.io/reml/bootstrap-runtime:local）
  -h, --help      このヘルプを表示

引数なしの場合は docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §9.3 で定義した
標準テストシーケンスを実行します。`--` 以降にコマンドを指定するとそのまま
コンテナ内で実行されます。

環境変数:
  CONTAINER_TOOL  docker または podman を強制指定（未設定時は自動検出）
  RUNTIME_TEST_JOBS  dune build/test の並列度（デフォルト: 4）
USAGE
}

resolve_container_tool() {
  if [[ -n "${CONTAINER_TOOL:-}" ]]; then
    echo "${CONTAINER_TOOL}"
    return
  fi

  if command -v podman >/dev/null 2>&1; then
    echo "podman"
  elif command -v docker >/dev/null 2>&1; then
    echo "docker"
  else
    echo "" && return 1
  fi
}

build_default_command() {
  cat <<'CMD'
set -euo pipefail
cd /workspace
if [[ ! -d .git ]]; then
  echo "このスクリプトはリポジトリのルートを /workspace にマウントして実行してください" >&2
  exit 1
fi

if [[ ! -d compiler/ocaml ]]; then
  echo "compiler/ocaml ディレクトリが見つかりません" >&2
  exit 1
fi

switch=5.2.1
eval "$(opam env --switch ${switch} --set-switch)"

export DUNE_CACHE=disabled
export DUNE_BUILD_JOBS="${RUNTIME_TEST_JOBS:-4}"

cd compiler/ocaml

opam exec -- dune build
opam exec -- dune runtest

# LLVM ゴールデンファイルを1件検証（基準: basic_arithmetic）
tmp_ll=$(mktemp /tmp/basic_arithmetic.XXXXXX.ll)
cp tests/llvm-ir/golden/basic_arithmetic.ll.golden "$tmp_ll"
chmod 644 "$tmp_ll"
scripts/verify_llvm_ir.sh "$tmp_ll"
rm -f "$tmp_ll"

opam exec -- make -C ../../runtime/native runtime
CMD
}

main() {
  local image_tag="ghcr.io/reml/bootstrap-runtime:local"

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -t|--tag)
        shift
        [[ $# -gt 0 ]] || { echo "エラー: --tag の直後にタグを指定してください" >&2; exit 1; }
        image_tag="$1"
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      --)
        shift
        break
        ;;
      *)
        echo "不明な引数: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
    shift
  done

  local tool
  tool=$(resolve_container_tool) || { echo "docker も podman も見つかりません" >&2; exit 1; }

  local repo_root
  repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)

  local workdir="${repo_root}"
  local cache_dir="${repo_root}/_docker_cache"
  mkdir -p "${cache_dir}"

  local -a run_cmd=("${tool}" "run" "--rm" "-e" "RUNTIME_TEST_JOBS=${RUNTIME_TEST_JOBS:-4}")

  if [[ "${tool}" == "docker" ]]; then
    run_cmd+=("-v" "${workdir}:/workspace")
    run_cmd+=("-v" "${cache_dir}:/workspace/_docker_cache")
  else
    run_cmd+=("-v" "${workdir}:/workspace:Z")
    run_cmd+=("-v" "${cache_dir}:/workspace/_docker_cache:Z")
  fi

  run_cmd+=("-w" "/workspace")
  run_cmd+=("${image_tag}")

  if [[ $# -gt 0 ]]; then
    local custom_cmd="$*"
    run_cmd+=("bash" "-lc" "${custom_cmd}")
  else
    local default_cmd
    default_cmd=$(build_default_command)
    run_cmd+=("bash" "-lc" "${default_cmd}")
  fi

  echo "[info] コンテナツール: ${tool}"
  echo "[info] 使用イメージ: ${image_tag}"
  printf '    %q' "${run_cmd[@]}"
  echo
  "${run_cmd[@]}"
}

main "$@"
