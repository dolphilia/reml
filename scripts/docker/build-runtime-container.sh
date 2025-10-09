#!/usr/bin/env bash
# Reml Phase 1 用 x86_64 Linux Docker イメージのビルドスクリプト
# docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §9 で定義した手順に対応する。

set -euo pipefail

usage() {
  cat <<'USAGE'
使用方法: scripts/docker/build-runtime-container.sh [-t <tag>] [--push] [--build-arg KEY=VALUE ...]

オプション:
  -t, --tag           ビルドするイメージタグ（デフォルト: ghcr.io/reml/bootstrap-runtime:local）
      --push          ビルド完了後に push を実行
      --build-arg     Dockerfile の --build-arg を追加（複数指定可）
  -h, --help          ヘルプを表示

環境変数:
  CONTAINER_TOOL      docker または podman を強制指定（未設定時は自動検出）
  USE_BUILDX          docker buildx を利用する場合に 1 を指定
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

main() {
  local image_tag="ghcr.io/reml/bootstrap-runtime:local"
  local push_flag=0
  local -a build_args=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -t|--tag)
        shift
        [[ $# -gt 0 ]] || { echo "エラー: --tag の直後にタグを指定してください" >&2; exit 1; }
        image_tag="$1"
        ;;
      --push)
        push_flag=1
        ;;
      --build-arg)
        shift
        [[ $# -gt 0 ]] || { echo "エラー: --build-arg の直後に KEY=VALUE を指定してください" >&2; exit 1; }
        build_args+=("--build-arg" "$1")
        ;;
      -h|--help)
        usage
        exit 0
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
  local dockerfile="${repo_root}/tooling/ci/docker/bootstrap-runtime.Dockerfile"

  if [[ ! -f "${dockerfile}" ]]; then
    echo "Dockerfile が見つかりません: ${dockerfile}" >&2
    exit 1
  fi

  echo "[info] コンテナツール: ${tool}"
  echo "[info] イメージタグ: ${image_tag}"

  local use_buildx=0
  local build_cmd=("${tool}" "build")
  if [[ "${tool}" == "docker" && "${USE_BUILDX:-0}" == "1" ]]; then
    use_buildx=1
    build_cmd=("${tool}" "buildx" "build")
  fi

  build_cmd+=("-f" "${dockerfile}" "-t" "${image_tag}")
  if [[ ${#build_args[@]} -gt 0 ]]; then
    build_cmd+=("${build_args[@]}")
  fi

  if [[ ${use_buildx} -eq 1 && ${push_flag} -eq 1 ]]; then
    build_cmd+=("--push")
  fi

  build_cmd+=("${repo_root}")

  echo "[info] ビルド開始..."
  printf '    %q' "${build_cmd[@]}"
  echo
  "${build_cmd[@]}"

  if [[ ${use_buildx} -eq 0 && ${push_flag} -eq 1 ]]; then
    echo "[info] イメージを push します"
    "${tool}" push "${image_tag}"
  fi

  echo "[info] 完了"
}

main "$@"
