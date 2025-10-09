#!/usr/bin/env bash
# macOS 上で生成した Linux x86_64 バイナリを QEMU で実行する補助スクリプト
# docs/plans/bootstrap-roadmap/1-5-runtime-integration.md §10.4 のフローを反映。

set -euo pipefail

usage() {
  cat <<'USAGE'
使用方法: scripts/cross/run-linux-qemu.sh [オプション] [-- <バイナリ引数>...]

主要オプション:
      --toolchain-home <PATH>  クロスツールチェーンディレクトリ（既定: tooling/toolchains/x86_64-unknown-linux-gnu）
      --sysroot <PATH>         sysroot ディレクトリ（既定: <toolchain-home>/sysroot）
      --binary <PATH>          実行する Linux バイナリ（既定: tooling/toolchains/examples/hello-linux）
      --build-cmd <CMD>        QEMU 実行前に実行するビルドコマンド（bash -lc で実行）
      --build-dir <PATH>       --build-cmd の作業ディレクトリ（未指定時はリポジトリルート）
      --qemu <PATH>            使用する QEMU 実行ファイル（既定: qemu-x86_64）
      --qemu-arg <ARG>         QEMU に追加する引数（複数指定可）
      --snapshot               QEMU を snapshot モードで実行
      --log-dir <PATH>         実行ログの保存先（既定: artifacts/cross）
      --metrics <PATH>         実行メトリクスを JSONL 形式で追記するファイル
      --dry-run                コマンドを表示するのみで実行しない
      --dump-env               計算された環境設定を表示して終了
  -h, --help                   このヘルプを表示

残りの引数（`--` 以降）は実行するバイナリへの引数として渡されます。
USAGE
}

log_info()  { echo "[info] $*"; }
log_warn()  { echo "[warn] $*" >&2; }
log_error() { echo "[error] $*" >&2; }

fail() {
  log_error "$@"
  exit 1
}

repo_root() {
  local script_dir
  script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
  cd "${script_dir}/../.." && pwd
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

main() {
  local repo
  repo=$(repo_root)

  local target_triple="${REML_TOOLCHAIN_TARGET:-x86_64-unknown-linux-gnu}"
  local toolchain_home="${repo}/tooling/toolchains/${target_triple}"
  local sysroot_path=""
  local binary_path=""
  local build_cmd=""
  local build_dir="${repo}"
  local qemu_bin="qemu-x86_64"
  local -a qemu_extra_args=()
  local snapshot_flag=0
  local log_dir="${repo}/artifacts/cross"
  local metrics_path=""
  local dry_run=0
  local dump_env=0
  local -a program_args=()

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --toolchain-home)
        shift || fail "--toolchain-home の直後にパスを指定してください"
        toolchain_home="$1"
        ;;
      --sysroot)
        shift || fail "--sysroot の直後にパスを指定してください"
        sysroot_path="$1"
        ;;
      --binary)
        shift || fail "--binary の直後にパスを指定してください"
        binary_path="$1"
        ;;
      --build-cmd)
        shift || fail "--build-cmd の直後にコマンド文字列を指定してください"
        build_cmd="$1"
        ;;
      --build-dir)
        shift || fail "--build-dir の直後にパスを指定してください"
        build_dir="$1"
        ;;
      --qemu)
        shift || fail "--qemu の直後にパスを指定してください"
        qemu_bin="$1"
        ;;
      --qemu-arg)
        shift || fail "--qemu-arg の直後に引数を指定してください"
        qemu_extra_args+=("$1")
        ;;
      --snapshot)
        snapshot_flag=1
        ;;
      --log-dir)
        shift || fail "--log-dir の直後にパスを指定してください"
        log_dir="$1"
        ;;
      --metrics)
        shift || fail "--metrics の直後にパスを指定してください"
        metrics_path="$1"
        ;;
      --dry-run)
        dry_run=1
        ;;
      --dump-env)
        dump_env=1
        ;;
      --)
        shift
        program_args=("$@")
        break
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

  if [[ ! -d "$toolchain_home" ]]; then
    log_warn "ツールチェーンディレクトリが存在しないためスキップします: ${toolchain_home}"
    exit 0
  fi

  if [[ -z "$sysroot_path" ]]; then
    sysroot_path="${toolchain_home}/sysroot"
  fi
  if [[ ! -d "$sysroot_path" ]]; then
    log_warn "sysroot ディレクトリが存在しないためスキップします: ${sysroot_path}"
    exit 0
  fi

  if [[ -z "$binary_path" ]]; then
    binary_path="${repo}/tooling/toolchains/examples/hello-linux"
  fi

  local env_file="${toolchain_home}/env.sh"
  if [[ -f "$env_file" ]]; then
    log_info "環境設定ファイル: ${env_file}"
  else
    log_warn "env.sh が見つかりません (${env_file})。PATH 設定は手動で確認してください。"
  fi

  log_info "QEMU 実行設定"
  log_info "  リポジトリ: ${repo}"
  log_info "  ツールチェーン: ${toolchain_home}"
  log_info "  sysroot: ${sysroot_path}"
  log_info "  バイナリ: ${binary_path}"
  log_info "  QEMU: ${qemu_bin}"

  if (( dump_env )); then
    log_info "--dump-env が指定されたため、ここで終了します。"
    exit 0
  fi

  if (( dry_run )); then
    log_info "dry-run モード: 実行コマンドのみを表示します。"
  fi

  if [[ -n "$build_cmd" ]]; then
    log_info "ビルドコマンドを実行します: ${build_cmd}"
    if (( dry_run )); then
      :
    else
      pushd "$build_dir" >/dev/null || fail "ビルドディレクトリに移動できません: ${build_dir}"
      bash -lc "$build_cmd"
      popd >/dev/null
    fi
  fi

  if [[ ! -x "$binary_path" ]]; then
    log_warn "実行可能ファイルが存在しないためスキップします: ${binary_path}"
    exit 0
  fi

  if ! command -v "$qemu_bin" >/dev/null 2>&1; then
    log_warn "$qemu_bin が見つからないため実行をスキップしました。brew install qemu などでインストールしてください。"
    exit 0
  fi

  mkdir -p "$log_dir"
  local timestamp
  timestamp=$(date -u +"%Y%m%dT%H%M%SZ")
  local stdout_log="${log_dir}/qemu-${timestamp}.out"
  local stderr_log="${log_dir}/qemu-${timestamp}.err"

  local ld_library_path="${sysroot_path}/lib:${sysroot_path}/lib64"
  local -a qemu_cmd=("$qemu_bin" "-L" "$sysroot_path")
  if (( snapshot_flag )); then
    qemu_cmd+=("-snapshot")
  fi
  if (( ${#qemu_extra_args[@]} > 0 )); then
    qemu_cmd+=("${qemu_extra_args[@]}")
  fi
  qemu_cmd+=("$binary_path")
  if (( ${#program_args[@]} > 0 )); then
    qemu_cmd+=("${program_args[@]}")
  fi

  log_info "QEMU コマンド:"
  printf '    %q' "${qemu_cmd[@]}"
  echo

  log_info "ログ出力先:"
  log_info "  stdout: ${stdout_log}"
  log_info "  stderr: ${stderr_log}"

  local exit_code=0
  local duration_sec=0

  if (( dry_run )); then
    log_info "dry-run のため QEMU は実行しません。"
  else
    SECONDS=0
    if ! (env QEMU_LD_PREFIX="$sysroot_path" LD_LIBRARY_PATH="$ld_library_path" \
        "${qemu_cmd[@]}" > >(tee "$stdout_log") 2> >(tee "$stderr_log" >&2)); then
      exit_code=$?
      log_error "QEMU 実行中にエラーが発生しました（exit=${exit_code}）"
    fi
    duration_sec=$SECONDS
    log_info "QEMU 実行時間: ${duration_sec} 秒"
  fi

  if [[ -n "$metrics_path" ]]; then
    mkdir -p "$(dirname "$metrics_path")"
    printf '{"timestamp":"%s","target":"%s","binary":"%s","duration_sec":%d,"exit_code":%d}\n' \
      "$timestamp" "$target_triple" "$binary_path" "$duration_sec" "$exit_code" >>"$metrics_path"
    log_info "メトリクスを追記しました: ${metrics_path}"
  fi

  if (( exit_code != 0 )); then
    exit "$exit_code"
  fi

  log_info "完了しました。"
}

main "$@"
