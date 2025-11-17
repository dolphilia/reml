# spec-integrity-audit-checklist 草案

> TODO: Phase 2-8 `spec-integrity-audit` 着手時に正式版へ昇格させる。現時点では Phase 2-5 ERR-001 の共有タスクで把握した監視項目のみを記録する。

## Phase 2-8 初動チェック（W36 更新）
- [x] `reports/spec-audit/` ディレクトリ構造（`ch0/`〜`ch3/`, `diffs/`, `summary.md`）を作成し、格納方針を `reports/spec-audit/README.md` に記載する。
- [x] `cargo test --manifest-path compiler/rust/frontend/Cargo.toml`（2025-11-17 実行）と `cargo run --manifest-path compiler/rust/frontend/Cargo.toml --bin poc_frontend -- --help` の成功ログを `reports/spec-audit/summary.md` に記録し、Rust Frontend を監査ベースラインとして利用できることを確認する。
- [x] Chapter 0/索引レビュー: `reports/spec-audit/ch0/links.md` にリンク検証結果と `docs/spec/0-0-overview.md` の差分要約を記録する（担当: Spec Core WG @ W36、2025-11-17 12:39 JST 更新）。
- [x] Chapter 1 サンプル実行: `cargo run ... --emit-diagnostics` の結果を `reports/spec-audit/ch1/` に保存し、`SYNTAX-002`/`SYNTAX-003` の `rust-gap` 状態を更新する（担当: Rust Parser WG @ W37 前半、2025-11-17 12:41 JST 実行ログを `reports/spec-audit/ch1/2025-11-17-syntax-samples.md` に集約）。
- [ ] Chapter 2 Streaming/Recover: `reports/spec-audit/ch2/streaming/*.json` に streaming runner の JSON を追加し、`ERR-001`/`ERR-002` の再現ログを添付する（担当: Parser API WG @ W37 後半）。
- [ ] Chapter 3 Diagnostics/Capability: `cargo test --manifest-path compiler/rust/runtime/ffi/Cargo.toml` と `compiler/rust/adapter/Cargo.toml` の結果、および `tooling/ci/collect-iterator-audit-metrics.py --section diagnostics --require-success` の出力を `reports/spec-audit/ch3/` に格納する（担当: Rust Runtime WG @ W38 前半）。
- [ ] `reports/spec-audit/diffs/` に `rust-gap` 向け差分メモ （フォーマット: `<ID>-<chapter>-rust-gap.md`）を作成し、`docs/plans/bootstrap-roadmap/2-5-spec-drift-remediation.md#phase28-diff-class` と相互参照する。`SYNTAX-002-ch1-rust-gap.md` を 2025-11-17 に追加済み（他 ID も同形式で追従する）。

## 期待集合（ERR-001）
- [ ] `parser.expected_summary_presence` が 1.0 を維持していることを `tooling/ci/collect-iterator-audit-metrics.py --require-success` で確認する。欠落した場合は `docs/plans/bootstrap-roadmap/2-5-review-log.md` 2025-11-16〜17 の手順を参照して検証をやり直す。
- [ ] `parser.expected_tokens_per_error` が 0.0 を下回らないことをチェックし、閾値を超える場合は `docs/spec/2-5-error.md` §B-7 の縮約ルールに従って上限設定を検討する。
- [ ] ストリーミング経路 (`docs/guides/core-parse-streaming.md` §3/§7) が `Diagnostic.expected` を CLI/LSP と同じ `ExpectationSummary` で公開しているか確認する。`StreamEvent::Error` で `ExpectedSummary` が欠落している場合は Phase 2-5 ERR-001 S5 の共有事項に沿って修正する。

## ドキュメント整合
- [ ] `docs/spec/2-5-error.md` と `docs/spec/3-6-core-diagnostics-audit.md` の脚注 `[^err001-phase25]` / `[^err001-phase25-core]` をレビューし、将来の仕様改訂で状態が変わった場合は脚注内容とリンクを更新する。

### `rust-gap` トラッキング表（2025-11-17 更新）
| 差分 ID | 章/カテゴリ | 症状 | Rust 監査手順 | 備考 |
|---------|-------------|------|----------------|------|
| `SYNTAX-002` | Chapter 1／モジュール `use` | 多段 `use` を Rust Parser で再現できるか未確認 | `cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml`（失敗ログ: `reports/spec-audit/ch1/use_nested-20251117-diagnostics.json`）。検証日は `YYYYMMDD` を付けて更新し、`reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` に dual-write トレースを保存。詳細は `reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` を参照 | OCaml 実装の暫定脚注を撤去する前提条件。Rust Frontend がファイル先頭の `module`/`use` を受理できるようにする（担当: Rust Parser WG / 期日: Phase 2-8 W37） |
| `SYNTAX-003` | Chapter 1／効果構文 | `perform`/`handle` の PoC 動作 | `cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml` の JSON を `reports/spec-audit/ch1/effect_handler-20251117-diagnostics.json` に保存 | KPI: `syntax.effect_construct_acceptance` が 1.0 であること。`-Zalgebraic-effects` 相当の CLI オプション設計も要確認 |
| `ERR-001` | Chapter 2／期待集合 | Streaming Recover で `ExpectedSummary` が欠落 | `tests/streaming_runner.rs` の `streaming_expected_token_snapshot_matches` を `reports/spec-audit/ch2/` へ転記 | `parser.expected_summary_presence` を 1.0 で維持 |
| `ERR-002` | Chapter 2／Recover Fix-it | `recover`/`fixit` 情報の Rust 実装反映状況 | `cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::recover -- --nocapture` ログ | CLI/LSP JSON の `recover.fixit.*` 拡張を Chapter 2 へ反映 |
| `CAP-001` | Chapter 3／Capability Stage | `BridgeAuditMetadata::as_json` が監査ログへ未接続 | `cargo test --manifest-path compiler/rust/runtime/ffi/Cargo.toml audit::tests::bridge_audit_metadata_as_json_serializes` の結果と `collect-iterator-audit-metrics.py --section diagnostics` | `reports/spec-audit/ch3/` に JSON を保存し、Stage 検査スクリプトと結合する |
