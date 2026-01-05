#!/bin/bash
# parse_throughput 指標を計測するベンチマークスクリプト
# Phase 1-6 Week 16: 10MB入力に対するパース時間を3回計測して平均値を出力

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
INPUT_FILE="${1:-$REPO_ROOT/examples/benchmark/large_input.reml}"
METRICS_FILE="${2:-/tmp/remlc-metrics.json}"
ITERATIONS="${3:-3}"

echo "=== Reml Parse Throughput Benchmark ==="
echo "Input file: $INPUT_FILE"
echo "Iterations: $ITERATIONS"
echo ""

# 入力ファイルが存在しない場合は生成
if [ ! -f "$INPUT_FILE" ]; then
  echo "Input file not found. Generating..."
  "$REPO_ROOT/tooling/examples/generate-large-input.sh" "$INPUT_FILE"
  echo ""
fi

# 入力サイズを確認
INPUT_SIZE=$(stat -f%z "$INPUT_FILE" 2>/dev/null || stat -c%s "$INPUT_FILE" 2>/dev/null)
echo "Input size: $((INPUT_SIZE / 1024 / 1024))MB ($INPUT_SIZE bytes)"
echo ""

# コンパイラパスを確認
REMLC="${REMLC:-opam exec -- dune exec -- remlc}"
echo "Compiler: $REMLC"
echo ""

# 3回計測
PARSE_TIMES=()
for i in $(seq 1 "$ITERATIONS"); do
  echo "Iteration $i/$ITERATIONS..."

  # メトリクスを生成
  $REMLC "$INPUT_FILE" --metrics "$METRICS_FILE" --metrics-format json >/dev/null 2>&1 || true

  # JSON からパース時間を抽出（jq を使用）
  if command -v jq &> /dev/null; then
    PARSE_TIME=$(jq -r '.phase_timings[] | select(.phase == "Parsing") | .elapsed_seconds' "$METRICS_FILE" 2>/dev/null || echo "0")
  else
    # jq がない場合は grep/sed で抽出
    PARSE_TIME=$(grep -A3 '"phase": "Parsing"' "$METRICS_FILE" | grep '"elapsed_seconds"' | sed 's/.*: \([0-9.]*\).*/\1/' || echo "0")
  fi

  PARSE_TIMES+=("$PARSE_TIME")
  echo "  Parse time: ${PARSE_TIME}s"
done

echo ""
echo "=== Results ==="

# 平均値を計算
TOTAL=0
for time in "${PARSE_TIMES[@]}"; do
  TOTAL=$(echo "$TOTAL + $time" | bc)
done
AVG=$(echo "scale=6; $TOTAL / $ITERATIONS" | bc)

echo "Parse times: ${PARSE_TIMES[*]}"
echo "Average parse time: ${AVG}s"

# parse_throughput を MB/s で計算
THROUGHPUT=$(echo "scale=2; ($INPUT_SIZE / 1024 / 1024) / $AVG" | bc)
echo "Parse throughput: ${THROUGHPUT} MB/s"

echo ""
echo "=== docs/guides/tooling/audit-metrics.md への記録用 ==="
echo "| parse_throughput | ${AVG}s (10MB入力) | フェーズごとに最低3回計測 |"
echo ""
echo "JSON メトリクスファイル: $METRICS_FILE"
