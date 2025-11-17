# SYNTAX-002 (Chapter 1 / module `use`) - rust-gap メモ

| 項目 | 内容 |
| --- | --- |
| ステータス | In Progress（Phase 2-8 W37 Rust Parser WG 担当） |
| 症状 | Rust Frontend が `docs/spec/1-1-syntax/examples/use_nested.reml` の `module`/`use` をトップレベルで受理できず、`構文エラー: 入力を解釈できません` を返す |
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
- 現状: `exit code 1`、`構文エラー: 入力を解釈できません`。`use_nested_rustcap.reml` のフォールバックでのみ成功。

## Phase 2-8 W37 アクション

1. **AST モデル整備**（Day 1）
   - `compiler/rust/frontend/src/parser/ast.rs` に `ModuleHeader`, `UseDecl`, `OperationDecl`, `HandlerDecl` を追加し、`TypeAnnot` を共有する `AnnotationKind` を導入。
   - `docs/plans/rust-migration/1-0-front-end-transition.md` / `p1-front-end-checklists.csv` に追補情報を記入。
2. **module_parser 改修**（Day 2）
   - `parse_module_header` → `parse_use_list` → `parse_decl_list` に分割し、`TraceEvent::ModuleHeaderAccepted` を挿入。
   - `scripts/poc_dualwrite_compare.sh use_nested` を `--emit-trace` 付きで再実行、OCaml/Rust dual-write 差分を確認。
3. **成果物更新**（Day 3）
   - `reports/spec-audit/ch1/use_nested-YYYYMMDD-diagnostics.json` を差し替え、`reports/spec-audit/ch1/2025-11-17-syntax-samples.md` に命名規約と `git rev-parse HEAD` を追記。
   - `docs/notes/spec-integrity-audit-checklist.md` の `SYNTAX-002` 行を `In Progress → Closed` へ変更、脚注で本ファイルを参照。

## クローズ条件

- `use_nested.reml` が診断 0 件で通過し、`use_nested_rustcap.reml` フォールバックを `docs/spec/1-1-syntax.md` から削除できる。
- `reports/spec-audit/ch1/use_nested-YYYYMMDD-trace.md` に dual-write トレースが残り、`TraceEvent::*Accepted` がすべて記録される。
- `docs/spec/0-3-code-style-guide.md` で案内する CLI 手順が Rust Frontend 版に更新され、`module`/`use` の動作例として `use_nested.reml` を参照できる。

## 参考リンク

- `reports/spec-audit/ch1/2025-11-17-syntax-samples.md`
- `docs/notes/spec-integrity-audit-checklist.md#rust-gap-トラッキング表2025-11-17-更新`
- `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md`
- `docs/plans/rust-migration/unified-porting-principles.md#同一観測点の再現`
