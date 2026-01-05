#!/bin/bash
# 10MB相当の大規模入力ファイルを生成するスクリプト
# Phase 1-6 Week 16: parse_throughput 指標の基準値測定用

set -euo pipefail

OUTPUT_FILE="${1:-examples/benchmark/large_input.reml}"
TARGET_SIZE=$((10 * 1024 * 1024))  # 10MB

echo "Generating large input file: $OUTPUT_FILE"
echo "Target size: $TARGET_SIZE bytes (10MB)"

# 出力ディレクトリを作成
mkdir -p "$(dirname "$OUTPUT_FILE")"

# ファイルヘッダー
cat > "$OUTPUT_FILE" << 'EOF'
// 自動生成されたベンチマークファイル
// Phase 1-6 Week 16: parse_throughput 測定用
// サイズ: 約10MB
//
// 生成日時: $(date)

EOF

# シンプルな関数定義を反復生成
FUNC_COUNT=0
CURRENT_SIZE=$(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null)

while [ "$CURRENT_SIZE" -lt "$TARGET_SIZE" ]; do
  # 関数定義を追加
  cat >> "$OUTPUT_FILE" << EOF
fn func_${FUNC_COUNT}() -> i64 = 42

EOF

  # 10関数ごとに少し複雑な関数を追加
  if [ $((FUNC_COUNT % 10)) -eq 0 ]; then
    cat >> "$OUTPUT_FILE" << EOF
fn add_${FUNC_COUNT}(a: i64, b: i64) -> i64 = a + b

fn conditional_${FUNC_COUNT}(x: i64) -> i64 =
  if x > 0 then x else 0

EOF
  fi

  FUNC_COUNT=$((FUNC_COUNT + 1))

  # サイズを更新
  CURRENT_SIZE=$(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null)

  # 進捗表示（1000関数ごと）
  if [ $((FUNC_COUNT % 1000)) -eq 0 ]; then
    echo "Generated $FUNC_COUNT functions, current size: $((CURRENT_SIZE / 1024))KB"
  fi
done

FINAL_SIZE=$(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null)
echo "Done! Generated $FUNC_COUNT functions"
echo "Final size: $((FINAL_SIZE / 1024 / 1024))MB ($FINAL_SIZE bytes)"
