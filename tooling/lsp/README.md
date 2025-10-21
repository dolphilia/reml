# tooling/lsp 概要（下書き）

Phase 2 以降で予定されている LSP/IDE 連携タスク（`docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` など）を追跡する領域です。診断・監査ポリシーと共通基盤を共有します。

## レイアウト（2025-10-24 更新）

- `diagnostic_transport.ml` — 現行 LSP PublishDiagnostics 生成ロジック（V2 ドラフト）。
- `lsp_transport.mli` / `lsp_transport.ml` — Phase 2-4 で導入予定の共通トランスポート API。
- `compat/diagnostic_v1.ml` — V1 互換層の分離先。CLI/LSP 双方が共通化する予定。
- `jsonrpc_server.ml` — PublishDiagnostics など JSON-RPC メッセージ生成の土台。
- `tests/client_compat/` — Vitest ベースの互換テスト。V1/V2 サンプル JSON を検証。

`diagnostic_serialization` モジュール（`compiler/ocaml/src/diagnostic_serialization.ml`）と連携し、CLI/LSP/CI が同じ中間表現を利用できるよう段階的に移行します。

## TODO
- [ ] LSP プロトコルの実験タスクが始まり次第、設計メモと実装手順を記載
- [ ] CLI 出力との整合チェックリストを整備
- [ ] Phase 3 `docs/plans/bootstrap-roadmap/3-6-core-diagnostics-audit-plan.md` に連動した診断ポリシーの実装 TODO を列挙
- [ ] `lsp_transport.mli` / `compat/diagnostic_v1.ml` の実装を埋め、V1/V2 両対応を完了する
