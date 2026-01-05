# 4-2 ドキュメント同期計画（Rust 移植 P4）

P4 フェーズでは Rust 実装の最終差分が仕様・ガイド・ノートへ確実に反映されているかを確認する必要がある。本書は [`docs/guides/tooling/audit-metrics.md`](../guides/tooling/audit-metrics.md) と `docs-migrations.log` を基点とし、Phase 3 Self-Host へ引き継げる情報状態を維持する手順をまとめる。

## 4-2.1 目的と適用範囲
- Rust 実装の最適化で生じる仕様差分（構文・型・効果・診断・Capability）を `docs/spec/` へ速やかに反映する。
- 運用・実装手順（CI、ガイド、ノート）が古い情報を参照し続けないよう、変更発生時に同期と脚注追記を行う。
- Phase 3 受入チームが `docs/spec/3-x` と実装の差異を追跡できるよう、更新履歴とレビュー記録を統一フォーマットで残す。

## 4-2.2 同期対象マトリクス
| カテゴリ | 主担当 | 更新条件 | 参照先 |
|----------|--------|----------|--------|
| 仕様 (`docs/spec/1-x`, `3-x`) | ドキュメントチーム | 構文・型推論・診断に影響するコード差分が生じた時 | `docs/spec/1-1-syntax.md`, `docs/spec/1-2-types-Inference.md`, `docs/spec/3-6-core-diagnostics-audit.md`, `docs/spec/3-8-core-runtime-capability.md` |
| ガイド (`docs/guides/`) | Rust 実装 + ドキュメント | CLI/CI 手順やランタイムブリッジの更新が必要な時 | `docs/guides/runtime/runtime-bridges.md`, `docs/guides/compiler/core-parse-streaming.md`, `docs/guides/ecosystem/ai-integration.md` |
| ノート (`docs/notes/`) | 各担当 | 意思決定・調査結果・残課題を共有する時 | `docs/notes/dsl/dsl-plugin-roadmap.md`, `docs/notes/stdlib/core-library-outline.md`, `docs/notes/phase3-handover/` |
| 計画書 (`docs/plans/`) | Rust 実装 + Phase 3 | フェーズ境界やハンドオーバー条件が変わった時 | `docs/plans/bootstrap-roadmap/2-7-to-2-8-handover.md`, `docs/plans/bootstrap-roadmap/3-0-phase3-self-host.md` |
| 用語集・付録 | ドキュメントチーム | 新用語・Rust 特有 API を導入した時 | `appendix/glossary-alignment.md`, `docs/spec/0-2-glossary.md` |

## 4-2.3 同期ワークフロー
1. **差分検出**: Rust 実装の PR で仕様に影響し得る変更が含まれる場合、PR テンプレートに「Documentation Impact」を記載し、対象ドキュメントを列挙する。
2. **更新作業**: 対象ドキュメントを更新し、変更点に脚注（例: `[^rust-p4-YYYYMMDD]`）を追加する。脚注では実装差分と関連 PR/メトリクスを紐付ける。
3. **レビュー & リンク整備**: `Docs Alignment Check`（[`4-1-communication-plan.md`](4-1-communication-plan.md) §4）でリンク切れ・用語揺れを確認し、必要に応じて `README.md` や `docs/spec/0-3-code-style-guide.md` へ導線を追加する。
4. **記録更新**: `docs-migrations.log` に更新対象・理由・関連リスクを追記し、`docs/guides/tooling/audit-metrics.md` の該当指標（例: `diagnostic.audit_presence_rate`）へ計測結果を転記する。
5. **フォローアップ**: 仕様差分が Phase 3 タスクへ波及する場合は `docs/notes/phase3-handover/` に TODO を残し、`4-0-risk-register.md` の該当リスクを更新する。

## 4-2.4 チェックリスト
- [ ] `collect-iterator-audit-metrics.py --section diagnostics --require-success`／`--section effects`／`--section streaming` の結果が `reports/audit/dashboard/` に反映されている。
- [ ] `docs/spec/` の該当章に Rust 実装差分を示す脚注が追加され、既存脚注との整合が取れている。
- [ ] `docs/guides/runtime/runtime-bridges.md`・`docs/guides/compiler/core-parse-streaming.md` の手順が最新 CI 設定と一致している。
- [ ] `appendix/glossary-alignment.md` と `docs/spec/0-2-glossary.md` に新しい用語が追記されている。
- [ ] `docs-migrations.log` へ更新記録が残され、`Docs Alignment Check` ミーティングでレビュー済みである。

## 4-2.5 監視指標とレビュー
- **同期 SLA**: 実装差分 Merge から 72 時間以内にドキュメント更新を完了する。期限を過ぎる場合は `Docs Alignment Check` で理由を共有し、`4-0-risk-register.md` の `P4-R3` を `Mitigating` に変更する。
- **品質ゲート**: `diagnostic.audit_presence_rate`, `stage_mismatch_count`, `typeclass.metadata_pass_rate` が CI で 1.0 を下回った場合は、差分ドキュメントを再チェックし、必要なら脚注で暫定措置を明記する。
- **レビューサイクル**: 週次で `docs/plans/rust-migration/README.md` のリンク整合を確認し、欠損があれば当日に修正する。

---

本計画は P4 作業期間中の標準フローとして扱い、対象範囲や SLA を変更する場合は `Docs Alignment Check` の議事録とあわせて改訂版を公開する。
