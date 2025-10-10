#!/usr/bin/env bash
# メモリ検証スクリプト (Phase 1-5)
#
# 使い方:
#   ./scripts/verify_memory.sh <executable>
#
# Valgrind または AddressSanitizer でメモリリーク・ダングリングポインタを検出

set -euo pipefail

if [ $# -lt 1 ]; then
  echo "使い方: $0 <executable>" >&2
  exit 1
fi

EXECUTABLE="$1"

if [ ! -f "$EXECUTABLE" ]; then
  echo "エラー: 実行ファイルが見つかりません: $EXECUTABLE" >&2
  exit 1
fi

echo "========================================="
echo "メモリ検証 (Phase 1-5)"
echo "========================================="
echo "対象: $EXECUTABLE"
echo ""

# Valgrind が利用可能か確認
if command -v valgrind >/dev/null 2>&1; then
  echo "[1/2] Valgrind によるリーク検出..."
  valgrind --leak-check=full --show-leak-kinds=all --error-exitcode=1 "$EXECUTABLE" 2>&1 | tee valgrind.log

  if grep -q "definitely lost: 0 bytes" valgrind.log; then
    echo "✓ メモリリークなし"
  else
    echo "✗ メモリリーク検出"
    exit 1
  fi
else
  echo "ℹ Valgrind が見つかりません。スキップします。"
fi

echo ""
echo "[2/2] 実行テスト..."
"$EXECUTABLE"
EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
  echo "✓ 正常終了"
else
  echo "✗ 異常終了 (exit code: $EXIT_CODE)"
  exit 1
fi

echo ""
echo "========================================="
echo "検証完了 ✓"
echo "========================================="
echo ""
echo "Phase 2 での拡張予定:"
echo "  - AddressSanitizer 統合"
echo "  - より詳細なリーク分析"
echo "  - ダングリングポインタ検出"
echo ""
