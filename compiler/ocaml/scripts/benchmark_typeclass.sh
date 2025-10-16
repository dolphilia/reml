#!/usr/bin/env bash
# Phase 2 Week 20-21: 型クラスベンチマーク自動計測スクリプト
# 辞書渡し vs モノモルフィゼーションPoC の性能比較

set -euo pipefail

# 色付き出力
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ディレクトリ設定
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPILER_DIR="$(dirname "$SCRIPT_DIR")"
BENCH_DIR="${COMPILER_DIR}/benchmarks"
OUTPUT_DIR="${COMPILER_DIR}/benchmark_results"
REMLC="${COMPILER_DIR}/_build/default/src/main.exe"

# 実行オプション
STATIC_ONLY=false
RUNS=3

# 引数解析
usage() {
    cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --static-only    実行フェーズをスキップし、静的指標のみを収集する
  --runs N         ベンチマーク実行回数（デフォルト: 3）
  -h, --help       このヘルプを表示
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --static-only)
            STATIC_ONLY=true
            shift
            ;;
        --runs)
            if [[ $# -lt 2 ]]; then
                echo "error: --runs requires an argument" >&2
                exit 1
            fi
            RUNS="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "error: unknown option '$1'" >&2
            usage
            exit 1
            ;;
    esac
done

# 出力ディレクトリ作成
mkdir -p "$OUTPUT_DIR/dictionary"
mkdir -p "$OUTPUT_DIR/monomorph"

# ログ関数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# コンパイラビルド確認
check_compiler() {
    log_info "コンパイラの存在確認..."
    if [ ! -f "$REMLC" ]; then
        log_error "コンパイラが見つかりません: $REMLC"
        log_info "dune build を実行してください"
        exit 1
    fi
    log_success "コンパイラを確認しました"
}

# ベンチマークファイルのコンパイル
compile_benchmark() {
    local bench_file="$1"
    local mode="$2"  # "dictionary" or "monomorph"
    local bench_name="$(basename "$bench_file" .reml)"
    local output_dir="$OUTPUT_DIR/$mode"

    log_info "コンパイル中: $bench_name ($mode モード)"

    # LLVM IR生成
    local ir_file="$output_dir/${bench_name}.ll"
    if ! "$REMLC" "$bench_file" --emit-ir --typeclass-mode="$mode" --out-dir="$output_dir" > /dev/null 2>&1; then
        log_warning "LLVM IR生成に失敗しました (継続します)"
    fi

    # ビットコード生成
    local bc_file="$output_dir/${bench_name}.bc"
    if ! "$REMLC" "$bench_file" --emit-bc --typeclass-mode="$mode" --out-dir="$output_dir" > /dev/null 2>&1; then
        log_warning "ビットコード生成に失敗しました (継続します)"
    fi

    if [ "$STATIC_ONLY" = true ]; then
        log_info "static-only: バイナリ生成をスキップします"
    else
        # 実行可能バイナリ生成
        local binary_file="$output_dir/${bench_name}"
        if ! "$REMLC" "$bench_file" --typeclass-mode="$mode" --out-dir="$output_dir" --link-runtime > /dev/null 2>&1; then
            log_error "バイナリ生成に失敗しました: $bench_name ($mode)"
            return 1
        fi
    fi

    log_success "コンパイル成功: $bench_name ($mode)"
    return 0
}

# ベンチマーク実行
run_benchmark() {
    local binary="$1"
    local runs="$2"
    local bench_name="$(basename "$binary")"

    log_info "ベンチマーク実行: $bench_name (${runs}回)"

    local total_time=0
    local times=()

    for i in $(seq 1 $runs); do
        # 実行時間計測
        local start=$(date +%s%N)
        if ! "$binary" > /dev/null 2>&1; then
            log_error "ベンチマーク実行に失敗しました: $binary"
            return 1
        fi
        local end=$(date +%s%N)

        local elapsed=$(( (end - start) / 1000000 )) # ナノ秒 -> ミリ秒
        times+=($elapsed)
        total_time=$((total_time + elapsed))

        log_info "  Run $i: ${elapsed}ms"
    done

    # 平均時間計算
    local avg_time=$((total_time / runs))
    echo "$avg_time"
}

# コードサイズ計測
measure_code_size() {
    local file="$1"
    if [ -f "$file" ]; then
        stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

# LLVM IR行数カウント
count_ir_lines() {
    local ir_file="$1"
    if [ -f "$ir_file" ]; then
        wc -l < "$ir_file" | tr -d ' '
    else
        echo "0"
    fi
}

# メモリ使用量計測（簡易版）
measure_memory() {
    local binary="$1"

    # macOSの場合はtime -l、Linuxの場合は/usr/bin/timeを使用
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS: time -lでメモリ計測
        local mem_info=$( (time -l "$binary" > /dev/null 2>&1) 2>&1 | grep "maximum resident set size")
        if [ -n "$mem_info" ]; then
            # バイト単位の値を抽出
            echo "$mem_info" | awk '{print $1}'
        else
            echo "0"
        fi
    else
        # Linux: /usr/bin/time -vでメモリ計測
        if command -v /usr/bin/time > /dev/null; then
            local mem_info=$(/usr/bin/time -v "$binary" 2>&1 | grep "Maximum resident set size")
            if [ -n "$mem_info" ]; then
                # KB単位の値を抽出してバイトに変換
                local kb=$(echo "$mem_info" | awk '{print $6}')
                echo $((kb * 1024))
            else
                echo "0"
            fi
        else
            log_warning "メモリ計測ツールが見つかりません"
            echo "0"
        fi
    fi
}

# JSON レポート生成
generate_json_report() {
    local output_file="$OUTPUT_DIR/benchmark_report.json"

    cat > "$output_file" << EOF
{
  "benchmark_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "phase": "Phase 2 Week 20-21",
  "compiler_version": "OCaml Bootstrap (Phase 1 complete)",
  "benchmarks": []
}
EOF

    echo "$output_file"
}

# 静的比較レポート生成
generate_static_report() {
    local output_file="$OUTPUT_DIR/static_comparison.json"
    local benchmarks_payload=""

    for entry in "${STATIC_RESULTS[@]}"; do
        if [ -n "$benchmarks_payload" ]; then
            benchmarks_payload+=",\n"
        fi
        benchmarks_payload+="    ${entry}"
    done

    cat > "$output_file" << EOF
{
  "benchmark_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "phase": "Phase 2 Week 20-21",
  "static_only": true,
  "benchmarks": [
${benchmarks_payload}
  ]
}
EOF

    log_success "静的比較レポート生成: $output_file"
}

# ベンチマーク比較レポート生成
generate_comparison_report() {
    local report_file="$OUTPUT_DIR/comparison_report.md"

    cat > "$report_file" << EOF
# 型クラス性能評価レポート

**日時**: $(date +"%Y-%m-%d %H:%M:%S")
**Phase**: Phase 2 Week 20-21
**目的**: 辞書渡し vs モノモルフィゼーションPoC の比較評価

## ベンチマーク結果

### 実行時間比較

| ベンチマーク | 辞書渡し (ms) | モノモルフィック (ms) | オーバーヘッド (%) |
|-------------|---------------|----------------------|-------------------|

### コードサイズ比較

| ベンチマーク | 辞書渡し (bytes) | モノモルフィック (bytes) | 増加率 (%) |
|-------------|-----------------|------------------------|------------|

### LLVM IR サイズ比較

| ベンチマーク | 辞書渡し (行) | モノモルフィック (行) | 増加率 (%) |
|-------------|--------------|---------------------|-----------|

### メモリ使用量比較

| ベンチマーク | 辞書渡し (KB) | モノモルフィック (KB) | 差異 (%) |
|-------------|--------------|---------------------|---------|

## 評価基準

- **実行時間オーバーヘッド**: 許容値 < 10%
- **コードサイズ増加率**: 許容値 < 30%
- **コンパイル時間**: 許容値 < 2倍
- **メモリ使用量**: 実行時メモリの差異

## 結論

(計測後に手動で追記)

EOF

    log_success "比較レポート生成: $report_file"
}

# 静的指標収集
collect_static_metrics() {
    local bench_name="$1"
    local mode="$2"
    local output_dir="$OUTPUT_DIR/$mode"
    local prefix="$output_dir/${bench_name}"

    local ir_file="${prefix}.ll"
    local bc_file="${prefix}.bc"
    local binary_file="${prefix}"

    local ir_lines
    ir_lines=$(count_ir_lines "$ir_file")
    local bc_size
    bc_size=$(measure_code_size "$bc_file")
    local binary_size
    binary_size=$(measure_code_size "$binary_file")

    cat <<EOF
{
  "ir_lines": ${ir_lines},
  "bitcode_size": ${bc_size},
  "binary_size": ${binary_size}
}
EOF
}

declare -a STATIC_RESULTS=()

# メイン処理
main() {
    log_info "=== 型クラスベンチマーク自動計測開始 ==="

    # コンパイラ確認
    check_compiler

    # ベンチマークファイルリスト
    local benchmarks=(
        "$BENCH_DIR/micro_typeclass.reml"
        "$BENCH_DIR/macro_typeclass.reml"
    )

    # 計測設定
    # 各ベンチマークについて辞書渡しとモノモルフィックの両方をコンパイル・実行
    for bench_file in "${benchmarks[@]}"; do
        if [ ! -f "$bench_file" ]; then
            log_warning "ベンチマークファイルが見つかりません: $bench_file"
            continue
        fi

        local bench_name="$(basename "$bench_file" .reml)"

        log_info "=== ベンチマーク: $bench_name ==="

        # 辞書渡し版のコンパイル
        if compile_benchmark "$bench_file" "dictionary"; then
            if [ "$STATIC_ONLY" = true ]; then
                log_info "static-only: 辞書渡し版 実行をスキップします"
            else
                # 実行時間計測
                local dict_binary="$OUTPUT_DIR/dictionary/$bench_name"
                if [ -x "$dict_binary" ]; then
                    local dict_time
                    dict_time=$(run_benchmark "$dict_binary" "$RUNS")
                    log_info "辞書渡し版 平均実行時間: ${dict_time}ms"

                    # コードサイズ計測
                    local dict_size
                    dict_size=$(measure_code_size "$dict_binary")
                    log_info "辞書渡し版 バイナリサイズ: ${dict_size} bytes"

                    # IR行数
                    local dict_ir_lines
                    dict_ir_lines=$(count_ir_lines "$OUTPUT_DIR/dictionary/${bench_name}.ll")
                    log_info "辞書渡し版 LLVM IR行数: ${dict_ir_lines}"
                fi
            fi
        fi

        # モノモルフィック版のコンパイル
        if compile_benchmark "$bench_file" "monomorph"; then
            if [ "$STATIC_ONLY" = true ]; then
                log_info "static-only: モノモルフィック版 実行をスキップします"
            else
                # 実行時間計測
                local mono_binary="$OUTPUT_DIR/monomorph/$bench_name"
                if [ -x "$mono_binary" ]; then
                    local mono_time
                    mono_time=$(run_benchmark "$mono_binary" "$RUNS")
                    log_info "モノモルフィック版 平均実行時間: ${mono_time}ms"

                    # コードサイズ計測
                    local mono_size
                    mono_size=$(measure_code_size "$mono_binary")
                    log_info "モノモルフィック版 バイナリサイズ: ${mono_size} bytes"

                    # IR行数
                    local mono_ir_lines
                    mono_ir_lines=$(count_ir_lines "$OUTPUT_DIR/monomorph/${bench_name}.ll")
                    log_info "モノモルフィック版 LLVM IR行数: ${mono_ir_lines}"
                fi
            fi
        fi

        if [ "$STATIC_ONLY" = true ]; then
            local dict_metrics
            dict_metrics=$(collect_static_metrics "$bench_name" "dictionary")
            local mono_metrics
            mono_metrics=$(collect_static_metrics "$bench_name" "monomorph")

            local bench_json
            bench_json=$(cat <<EOF
{
  "name": "${bench_name}",
  "dictionary": ${dict_metrics},
  "monomorph": ${mono_metrics}
}
EOF
)
            STATIC_RESULTS+=("$bench_json")
        fi

        echo ""
    done

    # レポート生成
    log_info "=== レポート生成 ==="
    if [ "$STATIC_ONLY" = true ]; then
        generate_static_report
    else
        generate_comparison_report
    fi

    log_success "=== ベンチマーク完了 ==="
    log_info "結果ディレクトリ: $OUTPUT_DIR"
    if [ "$STATIC_ONLY" = true ]; then
        log_info "静的比較レポート: $OUTPUT_DIR/static_comparison.json"
    else
        log_info "比較レポート: $OUTPUT_DIR/comparison_report.md"
    fi
}

# スクリプト実行
main "$@"
