#!/usr/bin/env bash
# クロスコンパイル済み Linux x86_64 バイナリを Docker コンテナ内で実行する補助スクリプト
# docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §9.3 の手順に対応する。

set -euo pipefail

usage() {
  cat <<'USAGE'
使用方法: scripts/docker/run-cross-binary.sh [-t <tag>] -- <binary path> [args...]

オプション:
  -t, --tag   使用するコンテナイメージタグ（デフォルト: ghcr.io/reml/bootstrap-runtime:local）
  -h, --help  このヘルプを表示

指定したバイナリはリポジトリ直下（例: artifacts/cross/hello-linux）のパスを想定します。
`scripts/docker/run-runtime-tests.sh` を用いてコンテナ内で実行し、exit code をホストへ伝搬します。
USAGE
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

  [[ $# -ge 1 ]] || { echo "エラー: 実行するバイナリを指定してください" >&2; usage >&2; exit 1; }

  local repo_root
  repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)

  local binary_path="$1"
  shift

  if [[ "${binary_path}" != /* ]]; then
    binary_path="${repo_root}/${binary_path}"
  fi

  if [[ "${binary_path}" != "${repo_root}"/* ]]; then
    echo "エラー: リポジトリ外のパスは指定できません (${binary_path})" >&2
    exit 1
  fi

  if [[ ! -f "${binary_path}" ]]; then
    echo "エラー: バイナリが見つかりません (${binary_path})" >&2
    exit 1
  fi

  local rel_path="${binary_path#${repo_root}/}"
  local container_path="/workspace/${rel_path}"

  local quoted_bin
  printf -v quoted_bin '%q' "${container_path}"

  local -a escaped_args=()
  for arg in "$@"; do
    local q
    printf -v q '%q' "$arg"
    escaped_args+=("$q")
  done

  local args_join=""
  if [[ ${#escaped_args[@]} -gt 0 ]]; then
    args_join=" ${escaped_args[*]}"
  fi

  local command="set -euo pipefail; cd /workspace; chmod +x ${quoted_bin}; ${quoted_bin}${args_join}"

  scripts/docker/run-runtime-tests.sh -t "${image_tag}" -- "${command}"
}

main "$@"
