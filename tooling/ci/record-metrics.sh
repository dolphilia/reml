#!/usr/bin/env bash
# CI メトリクス記録スクリプト（非推奨）
#
# GitHub Actions の CI 実行結果を docs/guides/tooling/audit-metrics.md に記録します。
# ただし当該ドキュメントは現状のリポジトリに存在しないため、見つからない場合は警告して終了します。
#
# 使い方:
#   ./tooling/ci/record-metrics.sh [オプション]
#
# オプション:
#   --target <TARGET>        ターゲットプラットフォーム（linux または macos、デフォルト: linux）
#   --build-time <TIME>      ビルド時間（例: "5m 32s"）
#   --test-count <COUNT>     テスト件数（例: "143"）
#   --test-result <RESULT>   テスト結果（success/failure）
#   --llvm-verify <RESULT>   LLVM 検証結果（success/failure）
#   --ci-run-id <ID>         GitHub Actions ランID
#   --timestamp <TIME>       タイムスタンプ（省略時は現在時刻）
#   --dry-run                実際には書き込まずに、出力内容のみ表示
#   -h, --help               このヘルプを表示
#
# 環境変数:
#   GITHUB_RUN_ID            GitHub Actions ランID（自動設定）
#   GITHUB_WORKFLOW          ワークフロー名（自動設定）
#   GITHUB_REF               ブランチ/タグ参照（自動設定）
#
# 参考:
#   - docs/guides/tooling/audit-metrics.md
#   - docs/plans/bootstrap-roadmap/1-7-linux-validation-infra.md

set -euo pipefail

# ========== 設定 ==========

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
METRICS_FILE="$REPO_ROOT/docs/guides/tooling/audit-metrics.md"

# デフォルト値
TARGET="linux"
BUILD_TIME="${BUILD_TIME:-unknown}"
TEST_COUNT="${TEST_COUNT:-unknown}"
TEST_RESULT="${TEST_RESULT:-unknown}"
LLVM_VERIFY="${LLVM_VERIFY:-unknown}"
CI_RUN_ID="${GITHUB_RUN_ID:-local}"
TIMESTAMP=""
DRY_RUN=0

# ========== ヘルパー関数 ==========

usage() {
  sed -n '1,26p' "$0"
}

log_info() {
  echo "[INFO] $*"
}

log_error() {
  echo "[ERROR] $*" >&2
}

# ========== 引数解析 ==========

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      shift || { log_error "--target の後に値を指定してください"; exit 1; }
      TARGET="$1"
      shift
      ;;
    --build-time)
      shift || { log_error "--build-time の後に値を指定してください"; exit 1; }
      BUILD_TIME="$1"
      shift
      ;;
    --test-count)
      shift || { log_error "--test-count の後に値を指定してください"; exit 1; }
      TEST_COUNT="$1"
      shift
      ;;
    --test-result)
      shift || { log_error "--test-result の後に値を指定してください"; exit 1; }
      TEST_RESULT="$1"
      shift
      ;;
    --llvm-verify)
      shift || { log_error "--llvm-verify の後に値を指定してください"; exit 1; }
      LLVM_VERIFY="$1"
      shift
      ;;
    --ci-run-id)
      shift || { log_error "--ci-run-id の後に値を指定してください"; exit 1; }
      CI_RUN_ID="$1"
      shift
      ;;
    --timestamp)
      shift || { log_error "--timestamp の後に値を指定してください"; exit 1; }
      TIMESTAMP="$1"
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

# ========== タイムスタンプ生成 ==========

if [[ -z "$TIMESTAMP" ]]; then
  if command -v date >/dev/null 2>&1; then
    # macOS と Linux の両方で動作する日付形式
    TIMESTAMP=$(date -u +"%Y-%m-%d %H:%M:%S UTC" 2>/dev/null || date +"%Y-%m-%d %H:%M:%S UTC")
  else
    TIMESTAMP="unknown"
  fi
fi

# ========== メトリクス記録内容の生成 ==========

# 結果の絵文字
if [[ "$TEST_RESULT" == "success" ]]; then
  RESULT_EMOJI="✅"
  RESULT_TEXT="成功"
else
  RESULT_EMOJI="❌"
  RESULT_TEXT="失敗"
fi

# LLVM 検証の絵文字
if [[ "$LLVM_VERIFY" == "success" ]]; then
  LLVM_EMOJI="✅"
  LLVM_TEXT="成功"
elif [[ "$LLVM_VERIFY" == "failure" ]]; then
  LLVM_EMOJI="❌"
  LLVM_TEXT="失敗"
else
  LLVM_EMOJI="⏳"
  LLVM_TEXT="未実行"
fi

# GitHub Actions のワークフロー情報
WORKFLOW_NAME="${GITHUB_WORKFLOW:-Unknown Workflow}"
BRANCH_NAME="${GITHUB_REF#refs/heads/}"
BRANCH_NAME="${BRANCH_NAME#refs/tags/}"

# ターゲット表示名
TARGET_DISPLAY="Linux"
if [[ "$TARGET" == "macos" ]]; then
  TARGET_DISPLAY="macOS"
fi

# 記録内容
METRICS_ENTRY="
### CI 実行結果 - $TARGET_DISPLAY（$TIMESTAMP）

- **ワークフロー**: $WORKFLOW_NAME
- **ブランチ**: $BRANCH_NAME
- **ターゲット**: $TARGET_DISPLAY
- **ラン ID**: $CI_RUN_ID
- **ビルド時間**: $BUILD_TIME
- **テスト件数**: $TEST_COUNT
- **テスト結果**: $RESULT_EMOJI $RESULT_TEXT
- **LLVM IR 検証**: $LLVM_EMOJI $LLVM_TEXT
"

# ========== Dry Run モード ==========

if (( DRY_RUN )); then
  log_info "Dry Run モード: 以下の内容が記録されます："
  echo "----------------------------------------"
  echo "$METRICS_ENTRY"
  echo "----------------------------------------"
  log_info "記録先: $METRICS_FILE"
  exit 0
fi

# ========== ファイルへの書き込み ==========

if [[ ! -f "$METRICS_FILE" ]]; then
  log_info "メトリクスファイルが見つかりません: $METRICS_FILE"
  log_info "このスクリプトは非推奨のため、書き込みをスキップします。"
  exit 0
fi

log_info "メトリクスを記録中..."
log_info "  ビルド時間: $BUILD_TIME"
log_info "  テスト件数: $TEST_COUNT"
log_info "  テスト結果: $RESULT_TEXT"
log_info "  LLVM 検証: $LLVM_TEXT"

# ファイル末尾に追記
echo "$METRICS_ENTRY" >> "$METRICS_FILE"

log_info "メトリクスを記録しました: $METRICS_FILE"
log_info "完了"

exit 0
