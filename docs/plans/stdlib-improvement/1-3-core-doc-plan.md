# 1.3 Core.Doc 計画

## 背景
- DSL が成長すると API ドキュメントが必須になるが、Reml 標準にドキュメント生成基盤がない。
- Doctest を通じて仕様と実装の同期を保つ仕組みが不足している。

## 目的
- ドキュメントコメントの抽出とレンダリングを標準化する。
- 例示コードの実行検証を通じて、ドキュメント品質を維持する。

## 仕様スコープ
- ドキュメントコメント（`///`）解析 API。
- HTML/Markdown 生成の最小 API とテンプレート規約。
- Doctest 実行ポリシー（成功条件/失敗時の診断）。

## 仕様更新
- 新設: `docs/spec/3-15-core-doc.md`
- 更新: `docs/spec/3-0-core-library-overview.md`
- 連携: `docs/spec/3-6-core-diagnostics-audit.md`（診断出力）

## ガイド/サンプル
- 新設: `docs/guides/dsl/doc-authoring.md`
- サンプル方針: DSL ライブラリのドキュメント生成例と Doctest 例を追加する。

## リスクと対策
- **出力差分の肥大化**: HTML テンプレートの差分を最小化し、テンプレート更新をバージョン管理する。
- **Doctest の実行負荷**: 実行モードとスキップ条件を段階化する。

## 成果物
- `Core.Doc` API の仕様ドラフト
- ドキュメント生成/Doctest の運用ルール案
