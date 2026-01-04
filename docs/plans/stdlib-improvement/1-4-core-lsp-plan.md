# 1.4 Core.Lsp 計画

## 背景
- DSL ファーストの方針では IDE 支援が不可欠だが、LSP 実装をゼロから作る負担が大きい。
- `Core.Parse` と `Core.Diagnostics` を LSP へ接続する標準手段が不足している。

## 目的
- LSP プロトコル型と JSON-RPC ループを標準ライブラリで提供し、DSL 作者が拡張できる基盤を用意する。
- エラー/補完/フォーマットなどの最小機能を **標準 API で組み立てられる** 状態にする。

## 仕様スコープ
- LSP 基本型（`Initialize`, `TextDocumentDidChange`, `Completion` など）。
- メッセージループと IO 連携（標準入出力、ストリーム）。
- 位置情報/テキスト操作のユーティリティ。

## 仕様更新
- 新設: `docs/spec/3-14-core-lsp.md`
- 更新: `docs/spec/3-0-core-library-overview.md`
- 連携: `docs/spec/3-5-core-io-path.md`, `docs/spec/3-6-core-diagnostics-audit.md`

## ガイド/サンプル
- 新設: `docs/guides/lsp/lsp-authoring.md`
- サンプル方針: JSON DSL の LSP サンプル（診断/補完）を `examples/` に置く。

## リスクと対策
- **プロトコル拡張の追随**: LSP バージョンを明記し、拡張は `Core.Lsp.Experimental` に隔離する。
- **性能と安定性**: 解析のキャンセル/再開ポリシーを `Core.Parse` のストリーミング指針と整合させる。

## 成果物
- `Core.Lsp` API の仕様ドラフト
- LSP 連携の最小テンプレート案
