# module_parser-20251119 Parser Tests

- Date: 2025-11-19 10:42 JST
- CI_RUN_ID: `rust-frontend-w37-20251119.1`
- Commit: `f9e10ae676bca22ed8a41e96d79f667310274990`
- Source docs: `docs/plans/bootstrap-roadmap/2-8-spec-integrity-audit.md#rust-frontend-パーサ拡張のステップ`, `docs/plans/rust-migration/3-0-ci-and-dual-write-strategy.md`, `docs/notes/spec-integrity-audit-checklist.md`

## 実行ログ

```
cargo test --manifest-path compiler/rust/frontend/Cargo.toml parser::module -- --nocapture
```

| テスト名 | サンプル | 期待診断 | 結果 | 備考 |
| --- | --- | --- | --- | --- |
| `module_header_accepts_use_nested` | `docs/spec/1-1-syntax/examples/use_nested.reml` | 0 件 | ✅ Pass | `TraceEvent::ModuleStageEntered(stage="Header")` が `use_nested-20251119-trace.md` に出力されたことを確認。 |
| `module_use_reports_shadowing` | `docs/spec/1-1-syntax/examples/use_nested.reml` | `syntax.use.shadowing` 1 件 | ✅ Pass | `UseDecl` の `alias` 重複を Rust 実装で再現。`reports/spec-audit/diffs/SYNTAX-002-ch1-rust-gap.md` へ再発防止条件を追記。 |
| `module_decl_accepts_effect_handler` | `docs/spec/1-1-syntax/examples/effect_handler.reml` | 0 件 | ✅ Pass | `DeclKind::Handler` が `module_parser` 直下で受理され、`TraceEvent::ModuleDeclAccepted(kind="handler")` を記録。 |
| `module_decl_reports_resume_without_operation` | `examples/effect_handler.reml` の `resume` 部分引用 | `effects.resume.unbound` 1 件 | ✅ Pass | `TypeAnnot::Resume` を共有し、`parser::module` レベルで診断化。 |
| `module_decl_blocks_roundtrip` | `docs/spec/1-1-syntax/examples/block_scope.reml` | 0 件 | ✅ Pass | `block_scope-20251119-diagnostics.json` を保存し、`TraceEvent::ModuleStageEntered(stage="DeclList")` を確認。 |
| `module_decl_dualwrite_snapshot` | `use_nested.reml`, `effect_handler.reml` | diff 0 | ✅ Pass | dual-write の参照結果を `reports/spec-audit/ch1/module_parser-20251119-dualwrite.md` に記録。 |

## トレースと保存先
- トレースファイル: `reports/spec-audit/ch1/use_nested-20251119-trace.md`, `block_scope-20251119-trace.md`, `effect_handler-20251119-trace.md`
- 診断 JSON: `reports/spec-audit/ch1/use_nested-20251119-diagnostics.json`, `block_scope-20251119-diagnostics.json`, `effect_handler-20251119-diagnostics.json`
- 参考: `docs/spec/1-1-syntax/examples/README.md` に保存ルールを追記（Phase 2-8 W38 で更新予定）

## メモ
- `ModuleStage` の 3 段階分割（Header / UseList / DeclList）を `parser::module` テストで直接検証できるよう、`TraceEvent::ModuleStageEntered` を `TraceSink` へ送り、`trace_id` を `syntax:module-stage::<stage>` に固定した。
- `tests/parser.rs` の統合ケースごとに `FixtureSample` を導入し、`docs/spec/0-3-code-style-guide.md` で定義した実行手順を Rust Frontend ベースに合わせた。
- CI の保存ルールは `reports/spec-audit/ch1/2025-11-17-syntax-samples.md#2025-11-19-module_parser-再実装ログ` を参照。
