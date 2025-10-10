#!/bin/bash
# ランタイム連携統合テスト
# Phase 1-5 LLVM 連携の動作確認

set -e

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
COMPILER_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
ROOT_DIR=$(cd "$COMPILER_DIR/../.." && pwd)
RUNTIME_LIB="$ROOT_DIR/runtime/native/build/libreml_runtime.a"

echo "========================================"
echo "ランタイム連携統合テスト (Phase 1-5)"
echo "========================================"
echo ""

# ランタイムライブラリの存在確認
if [ ! -f "$RUNTIME_LIB" ]; then
  echo "エラー: ランタイムライブラリが見つかりません: $RUNTIME_LIB"
  echo "runtime/native で 'make runtime' を実行してください"
  exit 1
fi

echo "✓ ランタイムライブラリ確認: $RUNTIME_LIB"
echo ""

# テンポラリディレクトリ作成
TEST_DIR=$(mktemp -d)
trap "rm -rf $TEST_DIR" EXIT

echo "テストディレクトリ: $TEST_DIR"
echo ""

# テスト1: 基本的な算術演算
echo "テスト1: 基本的な算術演算"
cat > "$TEST_DIR/test_basic.reml" <<'EOF'
fn add(a: i64, b: i64) -> i64 = a + b
fn main() -> i64 = add(2, 40)
EOF

echo "  コンパイル中..."
cd "$COMPILER_DIR"
opam exec -- dune exec -- remlc "$TEST_DIR/test_basic.reml" \
  --emit-ir \
  --out-dir "$TEST_DIR" \
  > "$TEST_DIR/test_basic.log" 2>&1 || {
  echo "  ✗ コンパイル失敗"
  cat "$TEST_DIR/test_basic.log"
  exit 1
}
cd - > /dev/null

echo "  ✓ LLVM IR 生成成功"
echo "  生成されたファイル:"
ls -lh "$TEST_DIR"/test_basic.* | awk '{print "    " $NF " (" $5 ")"}'
echo ""

# 生成されたIRの確認
if grep -q "declare.*@mem_alloc" "$TEST_DIR/test_basic.ll"; then
  echo "  ✓ ランタイム関数宣言確認: mem_alloc"
else
  echo "  ℹ mem_alloc 未使用（プリミティブ型のみ）"
fi

if grep -q "declare.*@print_i64" "$TEST_DIR/test_basic.ll"; then
  echo "  ✓ ランタイム関数宣言確認: print_i64"
else
  echo "  ℹ print_i64 未使用"
fi

echo ""
echo "========================================"
echo "全てのテスト成功！"
echo "========================================"
echo ""
echo "次のステップ:"
echo "  1. 文字列リテラルを含むテストを追加"
echo "  2. --link-runtime オプションで実行可能ファイル生成"
echo "  3. Valgrind/ASan でメモリリーク検証"
echo ""
