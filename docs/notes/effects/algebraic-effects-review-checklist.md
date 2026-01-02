# 代数的効果レビュー準備メモ

## 対象
- フェーズB 更新: `1-1-syntax.md`, `1-2-types-Inference.md`, `1-3-effects-safety.md`, `1-5-formal-grammar-bnf.md`
- フェーズC 更新: `2-5-error.md`, `2-6-execution-strategy.md`, `3-1-core-prelude-iteration.md`, `3-6-core-diagnostics-audit.md`, `3-8-core-runtime-capability.md`, `3-9-core-async-ffi-unsafe.md`

## アジェンダ
1. 構文/BNF と型推論の整合確認
2. 残余効果・Stage 管理の診断挙動
3. Async / FFI サンプルコードの妥当性
4. ガイド改訂内容とフェーズDタスクの確認

## レビュー資料
- 差分一覧: `git diff --stat`（フェーズB/C 関連ファイル）
- サンプル: `examples/algebraic-effects/` 配下の CLI・Capability・監査ログ例
- ステージ遷移チェック: `reml capability stage promote` コマンド手順

## スケジュール（案）
- 日時: 2025-11-05 14:00–16:00 JST
- 形式: オンライン（Teams）
- 参加者: フロントエンド担当 / 型システム担当 / ランタイム担当 / ドキュメント担当 / QA

## TODO（レビュー前）
- [ ] 各担当が担当章の差分を確認し、質問・懸念点をレビューコメントで共有
- [ ] サンプル実行ログを最新コミットで取得 (`make samples-algebraic-effects` 仮)
- [ ] レビュー議事録テンプレートを準備
