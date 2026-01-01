# 1.2 Core.Text.Pretty 計画

## 背景
- DSL のフォーマット/コード生成はテンプレートだけでは複雑なレイアウトに対応できない。
- `Core.Text` にプリティプリンタが無いため、各 DSL で独自実装が発生する。

## 目的
- Wadler-Leijen 系のプリティプリンタを標準化し、整形処理を再利用可能にする。
- AST からのフォーマット実装が最小コストで行えるよう、コンビネータ群を提供する。

## 仕様スコープ
- `text`, `line`, `softline`, `group`, `nest` などの基本コンビネータ。
- ページ幅とレイアウト選択の規則、評価戦略。
- AST の `Format` 相当インターフェイス（仮）と連携方法。

## 仕様更新
- 新設: `docs/spec/3-13-core-text-pretty.md`
- 更新: `docs/spec/3-0-core-library-overview.md`
- 連携: `docs/spec/3-3-core-text-unicode.md`（テキストモデル/グラフェム整合）

## ガイド/サンプル
- 新設: `docs/guides/dsl/formatter-authoring.md`
- サンプル方針: 代表的な DSL のフォーマッタ例と幅変更の比較出力を準備する。

## リスクと対策
- **性能劣化**: レイアウト探索の上限設定やストリーミング出力の指針を明文化する。
- **文字幅差**: Unicode 幅の扱いは `Core.Text.Unicode` と整合させる。

## 成果物
- `Core.Text.Pretty` API の仕様ドラフト
- レイアウト選択ルールとサンプル出力案
