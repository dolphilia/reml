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
| `SYNTAX-002` | Chapter 1／モジュール `use` | ✅ Close (2025-11-17) | `cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml --trace-output reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md`（診断 0 件。`reports/spec-audit/ch1/use_nested-20251117-diagnostics.json` と `use_nested-20251117-trace.md` を参照）。`reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` にクローズログを記載。 | `docs/spec/1-1-syntax.md` 脚注と `docs/spec/0-3-code-style-guide.md` を Rust Frontend ベースへ更新済み。`use_nested_rustcap.reml` は参考用途として残す。 |
| `SYNTAX-002/module_parser` | Chapter 1／module_parser 再実装 | `ModuleStage` 単位の再実装／TraceEvent ログ／統合テストの証跡不足 | `cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::module -- --nocapture` のログを `reports/spec-audit/ch1/module_parser-YYYYMMDD-parser-tests.md` に保存し、`scripts/poc_dualwrite_compare.sh use_nested`/`effect_handler` 実行結果を `module_parser-YYYYMMDD-dualwrite.md` へ追記する。 | 状態: `In Review (P2-8 W38)` 。owner: Parser QA。証跡: `reports/spec-audit/ch1/module_parser-20251119-parser-tests.md`（`CI_RUN_ID=rust-frontend-w37-20251119.1`）。 |
| `SYNTAX-003` | Chapter 1／効果構文 | ✅ Close (2025-11-18) | `cargo run --bin poc_frontend -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml --trace-output reports/spec-audit/ch1/effect_handler-20251118-trace.md`（診断 0 件、dual-write ログ `effect_handler-20251118-dualwrite.md`）。`reports/spec-audit/diffs/SYNTAX-003-ch1-rust-gap.md` 参照。 | `docs/plans/rust-migration/p1-rust-frontend-gap-report.md` と `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` で `Closed(P2-8)` と記録 |
| `SYNTAX-003/block_scope` | Chapter 1／ブロック/代入 | `let`/`var` の BindingKind とブロック式の JSON エビデンス | `reports/spec-audit/ch1/block_scope-20251118-diagnostics.json` / `block_scope-20251118-trace.md`（Rust Parser WG, due: W37 後半）。 | `BindingKind` と `TypeAnnot::Pending` を AST に維持すること。owner: Rust Parser WG |
| `SYNTAX-003/effect_handler` | Chapter 1／effect handler 診断 | dual-write で `effects.resume.untyped` が再現しないことの監査 | `reports/spec-audit/ch1/effect_handler-20251118-diagnostics.json` / `effect_handler-20251118-dualwrite.md`（owner: Effects WG, evidence(log) 記載）。 | KPI: `syntax.effect_construct_acceptance = 1.0`, `ci_run_id = rust-frontend-w37-20251118.2` |
| `SYNTAX-003/perform_do` | Chapter 1／perform/do フロー | `perform expr` と `do` ブロックの `EffectScopeId` 共有確認 | `cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::expr -- --nocapture` ログを `reports/spec-audit/ch1/perform_do-20251118-log.md`（準備中）へ保存 | owner: Rust Parser WG + Effects WG（due: W37 後半）。`TraceEvent::ExprEnter(kind=\"perform\")` を記録すること |
| `ERR-001` | Chapter 2／期待集合 | Streaming Recover で `ExpectedSummary` が欠落 | `tests/streaming_runner.rs` の `streaming_expected_token_snapshot_matches` を `reports/spec-audit/ch2/` へ転記 | `parser.expected_summary_presence` を 1.0 で維持 |
| `ERR-002` | Chapter 2／Recover Fix-it | `recover`/`fixit` 情報の Rust 実装反映状況 | `cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::recover -- --nocapture` ログ | CLI/LSP JSON の `recover.fixit.*` 拡張を Chapter 2 へ反映 |
| `CAP-001` | Chapter 3／Capability Stage | `BridgeAuditMetadata::as_json` が監査ログへ未接続 | `cargo test --manifest-path compiler/rust/runtime/ffi/Cargo.toml audit::tests::bridge_audit_metadata_as_json_serializes` の結果と `collect-iterator-audit-metrics.py --section diagnostics` | `reports/spec-audit/ch3/` に JSON を保存し、Stage 検査スクリプトと結合する |
