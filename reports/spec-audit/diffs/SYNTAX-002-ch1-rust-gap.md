# SYNTAX-002 (Chapter 1 / module `use`) - rust-gap メモ

| 項目 | 内容 |
| --- | --- |
| ステータス | Closed（2025-11-17、Rust Parser WG） |
| 症状 | `docs/spec/1-1-syntax/examples/use_nested.reml` の `module`/`use`/`fn ... { ... }`/`match` を Rust Frontend が受理し、診断 0 件で完了する |
| 期待結果 | OCaml 実装と同様に `module sample.core` + 多段 `use` を受理し、診断 0 件の JSON を `reports/spec-audit/ch1/use_nested-YYYYMMDD-diagnostics.json` に保存できる |
| 参照仕様 | `docs/spec/1-1-syntax.md` §2 (Module/Use), `docs/spec/0-3-code-style-guide.md` §2 (サンプル実行手順) |
| 関連計画 | `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md#rust-frontend-パーサ拡張のステップ`, `docs/plans/rust-migration/1-0-front-end-transition.md#phase-2-8-追補w37-moduleheader--usedecl-トップレベル整備` |

## 再現手順

```bash
cargo run --manifest-path compiler/rust/frontend/Cargo.toml \
  --bin poc_frontend \
  -- --emit-diagnostics docs/spec/1-1-syntax/examples/use_nested.reml \
  --trace-output reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md
```

- 期待: `exit code 0`、診断出力 0 件、`TraceEvent::ModuleHeaderAccepted`/`TraceEvent::UseDeclAccepted` が `use_nested-YYYYMMDD-trace.md` に記録される。
- 現状: `exit code 0`、診断 0 件。`TraceEvent::{ModuleHeaderAccepted,UseDeclAccepted}` を `reports/spec-audit/ch1/use_nested-20251117-trace.md` に記録し、`reports/spec-audit/ch1/use_nested-20251117-diagnostics.json` に成功ログを保存済み。

## Phase 2-8 W37 アクション

1. **AST モデル整備**（Day 1）✅
   - `compiler/rust/frontend/src/parser/ast.rs` に `ModuleHeader` / `UseDecl` / `OperationDecl` / `HandlerDecl` を追加し、`TypeAnnot` の `AnnotationKind` を導入済み。
   - `docs/plans/rust-migration/1-0-front-end-transition.md` / `p1-front-end-checklists.csv` に追補情報を記入する。
2. **module_parser 改修**（Day 2）✅
   - `parse_module_header` → `parse_use_list` → `parse_decl_list` に分割し、`TraceEvent::ModuleHeaderAccepted` / `TraceEvent::UseDeclAccepted` を挿入済み。
   - `cargo run ... --trace-output reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` で `TraceEvent::*Accepted` を保存できるようになった。
3. **ブロック/`match` 実装**（Day 3）✅
   - `compiler/rust/frontend/src/parser/mod.rs` にブロックステートメント (`let`/expr)、`match` パターン、`::`/`.` フィールドアクセス、`KeywordNew` 呼び出しを実装。
   - `reports/spec-audit/ch1/use_nested-20251117-diagnostics.json` / `use_nested-20251117-trace.md` を診断 0 件の成果で更新し、`docs/notes/spec-integrity-audit-checklist.md` で `SYNTAX-002` を `Closed` に移行。

## クローズ条件

- `use_nested.reml` が診断 0 件で通過し、`reports/spec-audit/ch1/use_nested-YYYYMMDD-diagnostics.json` / `use_nested-YYYYMMDD-trace.md` を最新化する。✅
- `docs/spec/1-1-syntax.md` 脚注と `docs/spec/0-3-code-style-guide.md` を Rust Frontend ベースへ更新する。✅
- `docs/notes/spec-integrity-audit-checklist.md` と本ファイルを `Closed` 扱いにし、`rust-gap SYNTAX-002` を Phase 2-8 で完了したと記録する。✅

## Streaming acceptance (2025-11-21)

- Streaming Runner の統合テスト `compiler/rust/frontend/tests/streaming_metrics.rs` に `module_header_acceptance` / `effect_handler_acceptance` / `bridge_signal_roundtrip` を追加し、`StreamFlowState::latest_bridge_signal()` が `Option<RuntimeBridgeSignal>` を直接返すことを `assert_matches!` で拘束。ログは `reports/spec-audit/ch1/streaming_metrics-20251121-log.md` に `CI_RUN_ID=rust-frontend-streaming-20251121.1` と `git rev-parse HEAD = 3c92026356502383863dee228220ecdf02c24fd8` を含めて保存。
- `reports/spec-audit/ch1/streaming_use_nested-20251121-diagnostics.json` / `streaming_effect_handler-20251121-diagnostics.json` を作成し、`mode = streaming` とサンプルごとの `ci_run_id`・`git_rev` を付記。複製を `reports/spec-audit/ch2/streaming/` に配置し、`docs/notes/spec-integrity-audit-checklist.md#期待集合err-001` の指標と連動させた。
- `docs/spec/1-1-syntax.md`、`docs/spec/0-3-code-style-guide.md`、`docs/plans/rust-migration/overview.md`、`docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md` に Streaming ベースライン確立の脚注・計画更新を反映し、`use_nested_rustcap.reml` を監査ベースラインから除外した。

## 参考リンク

- `reports/spec-audit/ch1/2025-11-17-syntax-samples.md`
- `docs/notes/spec-integrity-audit-checklist.md#rust-gap-トラッキング表2025-11-17-更新`
- `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`
- `docs/plans/rust-migration/unified-porting-principles.md#同一観測点の再現`
