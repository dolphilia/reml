# SYNTAX-002 (Chapter 1 / module `use`) - rust-gap メモ

| 項目 | 内容 |
| --- | --- |
| ステータス | In Progress（Phase 2-8 W37 Rust Parser WG 担当。`module`/`use` は受理済み、ブロック構文が未実装） |
| 症状 | `docs/spec/1-1-syntax/examples/use_nested.reml` の `fn ... { ... }` 本体を Rust Frontend が解析できず、`let` 行で `構文エラー: 入力を解釈できません` を返す |
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
- 現状: `exit code 0` で `TraceEvent::{ModuleHeaderAccepted,UseDeclAccepted}` は取得できたが、ブロック構文 (`fn ... { ... }`) を解析できず `use_nested.reml` の `let` 行で `構文エラー` が残る。`reports/spec-audit/ch1/use_nested_rustcap-YYYYMMDD-diagnostics.json` フォールバックは引き続き成功。

## Phase 2-8 W37 アクション

1. **AST モデル整備**（Day 1）✅
   - `compiler/rust/frontend/src/parser/ast.rs` に `ModuleHeader` / `UseDecl` / `OperationDecl` / `HandlerDecl` を追加し、`TypeAnnot` の `AnnotationKind` を導入済み。
   - `docs/plans/rust-migration/1-0-front-end-transition.md` / `p1-front-end-checklists.csv` に追補情報を記入する。
2. **module_parser 改修**（Day 2）✅
   - `parse_module_header` → `parse_use_list` → `parse_decl_list` に分割し、`TraceEvent::ModuleHeaderAccepted` / `TraceEvent::UseDeclAccepted` を挿入済み。
   - `cargo run ... --trace-output reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` で `TraceEvent::*Accepted` を保存できるようになった。
3. **成果物更新**（Day 3）⏳
   - `reports/spec-audit/ch1/use_nested-YYYYMMDD-diagnostics.json` は `fn` ブロック未対応のログを差し替え済み。`git rev-parse HEAD` の追記とブロック構文タスクの切り出しが未了。
   - `docs/notes/spec-integrity-audit-checklist.md` の `SYNTAX-002` 行は `In Progress` のまま、ブロック構文対応チケットを追加する。

## クローズ条件

- `use_nested.reml` が診断 0 件で通過し、`use_nested_rustcap.reml` フォールバックを `docs/spec/1-1-syntax.md` から削除できる（ブロック構文を Rust Frontend が受理できること）。
- `reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` に dual-write トレースが残り、`TraceEvent::*Accepted` がすべて記録される。
- `docs/spec/0-3-code-style-guide.md` で案内する CLI 手順が Rust Frontend 版に更新され、`module`/`use` の動作例として `use_nested.reml` を参照できる。

## 参考リンク

- `reports/spec-audit/ch1/2025-11-17-syntax-samples.md`
- `docs/notes/spec-integrity-audit-checklist.md#rust-gap-トラッキング表2025-11-17-更新`
- `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`
- `docs/plans/rust-migration/unified-porting-principles.md#同一観測点の再現`
