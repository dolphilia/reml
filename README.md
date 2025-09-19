# Kestrel 仕様書

このプロジェクトには、パーサーコンビネーターに最適化された言語である Kestrel の仕様書がまとめられています。

## 目次

- はじめに
  - [概要](0-1-overview.md)
  - [ロジェクトの目的と指針](0-2-project-purpose.md)
- 言語コア仕様
  - [構文](1-1-syntax.md)
  - [型と推論](1-2-types-Inference.md)
  - [効果と安全性](1-3-effects-safety.md)
  - [文字モデル](1-4-test-unicode-model.md)
- 標準API仕様
  - [パーサ型](2-1-perser-type.md)
  - [コア・コンビネータ](2-2-core-combinator.md)
  - [字句レイヤ](2-3-lexer.md)
  - [演算子優先度ビルダー](2-4-op-builder.md)
  - [エラー設計](2-5-error.md)
  - [実行戦略](2-6-execution-strategy.md)
- 付録
  - [BNF](3-1-bnf.md)
  - [LLVMとの連携](a-jit.md)
  - [最初のアイデア](b-first-idea.md)
