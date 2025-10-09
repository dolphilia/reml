#!/usr/bin/env bash
# Phase 1 用 x86_64 Linux スモークテスト — Docker コンテナ内で最小動作確認を実施

set -euo pipefail

usage() {
  cat <<'USAGE'
使用方法: scripts/docker/smoke-linux.sh [-t <tag>]

docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §9.4 に基づき、
以下のシーケンスで動作確認を行います:
  1. dune build で remlc をビルド
  2. ランタイムライブラリを make でビルド
  3. remlc --emit-ir --verify-ir を用いて examples/language-impl-comparison/reml/basic_interpreter.reml
     をコンパイルし、生成した IR を _docker_cache/smoke/ に配置

オプション:
  -t, --tag   実行するコンテナイメージタグ（既定: ghcr.io/reml/bootstrap-runtime:local）
  -h, --help  このヘルプを表示
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
      *)
        echo "不明な引数: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
    shift
  done

  local smoke_command
smoke_command=$(cat <<'CMD'
set -euo pipefail
cd /workspace
switch=5.2.1
eval "$(opam env --switch ${switch} --set-switch)"

mkdir -p /workspace/_docker_cache/smoke

cd compiler/ocaml

opam exec -- dune build src/main.exe
opam exec -- make -C ../../runtime/native runtime

opam exec -- dune exec -- remlc \
  --emit-ir \
  --verify-ir \
  --out-dir /workspace/_docker_cache/smoke \
  ../../examples/language-impl-comparison/reml/basic_interpreter.reml
CMD
)

  RUNTIME_TEST_JOBS=${RUNTIME_TEST_JOBS:-2} \
    scripts/docker/run-runtime-tests.sh -t "${image_tag}" -- "${smoke_command}"
}

main "$@"
