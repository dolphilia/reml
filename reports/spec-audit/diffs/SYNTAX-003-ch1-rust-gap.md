# SYNTAX-003 (Chapter 1 / effect handler) - rust-gap メモ

| 項目 | 内容 |
| --- | --- |
| ステータス | Closed（2025-11-18、Rust Parser WG + Effects WG） |
| 症状 | `docs/spec/1-1-syntax/examples/effect_handler.reml` の `effect` / `operation` / `handle ... with handler` を Rust Frontend が受理し `diagnostics` 0 件で終了すること |
| 期待結果 | OCaml 実装と同様に `perform Console.log` と `handle expr with handler { operation log(args, resume) { ... } }` が受理され、`reports/spec-audit/ch1/effect_handler-YYYYMMDD-diagnostics.json` が空診断で保存される |
| 参照仕様 | `docs/spec/1-1-syntax.md` §4.1〜§5, `docs/spec/1-3-effects-safety.md` §3（effect scope / resume） |
| 関連計画 | `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md#rust-frontend-パーサ拡張のステップ`, `docs/plans/rust-migration/1-0-front-end-transition.md#phase-2-8-追補w37-exprparser--effect-handler`, `docs/plans/rust-migration/1-2-diagnostic-compatibility.md#effect-handler-acceptance` |

## 再現手順

```bash
cargo run --manifest-path compiler/rust/frontend/Cargo.toml \
  --bin poc_frontend \
  -- --emit-diagnostics docs/spec/1-1-syntax/examples/effect_handler.reml \
  --trace-output reports/spec-audit/ch1/effect_handler-YYYYMMDD-trace.md
```

- 期待: `exit code 0`、診断 0 件。`TraceEvent::ExprEnter`/`ExprLeave` に `handle`, `operation`, `perform` が記録され、`EffectExprKind` が `trace_id=syntax:expr-handle` で保存される。
- 現状: 2025-11-18 の実行で `diagnostics` 0 件。`TraceEvent::{ExprEnter,ExprLeave}` が `effect_handler-20251118-trace.md`（`reports/spec-audit/ch1/`）に記録され、`TypeAnnot::Resume` が `resume` 引数へ付与されていることを `diagnostic trace` で確認。

## Phase 2-8 W37 アクション

1. **ExprParser 導入**（Day 1）✅  
   - `compiler/rust/frontend/src/parser/expr.rs` を追加し、`module_parser.rs` からブロック/式要素を分離。`Expr` 列挙に `Block`, `Let`, `Do`, `Perform`, `Handle`, `Resume`, `Return` を追加した。`TraceEvent::{ExprEnter,ExprLeave}` と `trace_id=syntax:expr-*` を導入。
2. **block_scope 検証**（Day 2 午前）✅  
   - `docs/spec/1-1-syntax/examples/block_scope.reml` を CLI で実行し、`reports/spec-audit/ch1/block_scope-20251118-diagnostics.json` に診断 0 件で保存。`BindingKind::{Immutable, Mutable}` を AST に追加し、`TypeAnnot::Pending` を let/var の双方に付与した。
3. **effect handler/operation 実装**（Day 2 午後）✅  
   - `EffectExprKind` を導入し `perform` / `handle` / `operation` を統合。`OperationDecl` と `HandlerDecl` が `TypeAnnot::Resume` を共有するように `ast.rs` を拡張し、`compiler/rust/frontend/src/diagnostics/mod.rs` に `effects.resume.untyped` の Rust 版ロジックを追加。
4. **dual-write 比較とログ**（Day 3）✅  
   - `scripts/poc_dualwrite_compare.sh effect_handler` を追加実行し、OCaml/Rust の診断 JSON が一致することを `reports/spec-audit/ch1/effect_handler-20251118-dualwrite.md` に記録。`reports/spec-audit/ch1/effect_handler-20251118-diagnostics.json` を保存し、`docs/notes/process/spec-integrity-audit-checklist.md#rust-gap-トラッキング表2025-11-17-更新` を更新。

## W39 Trace Coverage 追加

- `parser/mod.rs` に `ParserTraceEventKind::{ExprEnter,ExprLeave,EffectEnter,EffectExit,HandlerAccepted,OperationResume}` を追加し、`syntax:expr::<kind>` / `syntax:effect::<kind>` / `syntax:handler::<name>` / `syntax:operation::resume` で `trace_id` を固定。`FrontendDiagnostic.extensions.trace_ids` を `build_parser_diagnostics` で拡張し、診断とトレースを 1:多で紐付けられるようにした。
- `reports/spec-audit/ch1/trace-coverage-20251122.md` に `scripts/poc_dualwrite_compare.sh effect_handler --trace` の実行コマンド、`CI_RUN_ID`、`git rev-parse HEAD`、および `Trace coverage >= 4`（handle / perform / resume / block）を満たす証跡をまとめた。`effect_handler-20251118-trace.md` / `block_scope-20251118-trace.md` の `syntax:expr::<kind>` が `trace_ids` 配列と一致していることを確認し、本メモへリンク。

## クローズ条件

- `effect_handler.reml` が診断 0 件で通過し、`reports/spec-audit/ch1/effect_handler-YYYYMMDD-diagnostics.json` / `effect_handler-YYYYMMDD-dualwrite.md` を添付。✅
- `docs/spec/1-1-syntax.md` §5 脚注からフォールバックを削除し、`reports/spec-audit/ch1/effect_handler-YYYYMMDD-diagnostics.json` への参照を明記。✅
- `docs/plans/rust-migration/p1-rust-frontend-gap-report.md` の `SYNTAX-003` 行を `Closed (P2-8)` に更新し、`docs/plans/rust-migration/overview.md` に `effect_handler.reml` 受理を Phase 1 完了条件として記載。✅

## 参考リンク

- `reports/spec-audit/ch1/block_scope-20251118-diagnostics.json`
- `reports/spec-audit/ch1/effect_handler-20251118-diagnostics.json`
- `reports/spec-audit/ch1/effect_handler-20251118-dualwrite.md`
- `reports/spec-audit/ch1/2025-11-17-syntax-samples.md#2025-11-18-追加サンプル`
- `docs/notes/process/spec-integrity-audit-checklist.md#rust-gap-トラッキング表2025-11-17-更新`
