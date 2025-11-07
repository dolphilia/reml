# W3 型推論 Dual-write 実行計画

W3（型推論コア移植）で求められる制約生成・ソルバ移植・診断検証を、再現性のある dual-write 実行フローとして整理する。`1-0-front-end-transition.md#W3` のステップ 3 で定義した作業を具体化し、成果物の場所とテスト入力セットを共有する。

## 1. 目的と成果物
- OCaml 実装 (`type_inference.ml`, `constraint_solver.ml`, `impl_registry.ml`) と Rust 実装 (`compiler/rust/frontend/src/typeck/`) が生成する Typed AST / Constraint / Effect / Impl Registry 情報を 1:1 で比較できるログを出力する。
- `reports/dual-write/front-end/w3-type-inference/<case>/` 以下に以下の成果物を保存する命名規約を定義する。

| 種別 | ファイル名 | 内容 | 備考 |
| --- | --- | --- | --- |
| Typed AST | `typed-ast.{ocaml,rust}.json` | `infer_decl` などの結果を JSON 化 | `jq --sort-keys` 済み |
| Constraints | `constraints.{ocaml,rust}.json` | `ConstraintSet`/`Scheme` のスナップショット | `Type_inference.dump_constraints` を OCaml 側で再利用 |
| Impl Registry | `impl-registry.{ocaml,rust}.json` | 型クラス辞書の登録順序・キーセット | `RwLock<IndexMap<..>>` で determinism を担保 |
| Effect Metrics | `effects-metrics.{ocaml,rust}.json` | `Type_inference_effect` / `effects.*` メトリクス | `collect-iterator-audit-metrics.py --section effects` の結果 |
| Typeck Debug | `typeck-debug.{ocaml,rust}.json` | 失敗ケースの `effect_scope` / `residual_effects` / `recoverable` | CLI `--emit typeck-debug <dir>` で出力 |
| Summary | `summary.json` / `summary.md` | ケースごとの比較結果 | `scripts/poc_dualwrite_compare.sh` が自動生成 |

## 2. 制約生成とソルバ移植の粒度
1. **制約生成 (`infer_expr`/`infer_pattern`/`infer_decl`)**  
   - Rust 側では `compiler/rust/frontend/src/typeck/constraint.rs` に `ConstraintBuilder` を定義し、OCaml `Constraint.new_constraint` 呼び出しに対応する API を列挙。  
   - `TypedExpr` から `ConstraintSnapshot` への写像を `typed_ast_schema_draft.md` の `TyId` 規約に従って `u32` ID で保持し、Dual-write JSON に `ty_id` / `origin_span` / `stage_requirement` を含める。
2. **Constraint / Scheme のシリアライズ**  
   - `Scheme` 汎化部 (`forall`) を `Vec<TyVar>`（Rust）と `string list`（OCaml）で対応付け、JSON では `["a0","a1",...]` の形式に正規化。  
   - `ConstraintSet` には `kind`, `lhs`, `rhs`, `evidence` を必須キーとして持たせ、`type_row`/`effect_row` の残余情報は `extensions` に格納する。
3. **ソルバ (`Constraint_solver.unify` 相当)**  
   - `occurs_check_failed`, `stage_mismatch`, `effect_row_unify_failed` など代表的なエラーコードを `diagnostic::codes::TYPE_*` へマッピングし、`Result` 化後も OCaml の例外名を `ocaml_error` フィールドに保持。  
   - `solver.rs` で `UnifyStats` を導入し、`reports/dual-write/front-end/w3-type-inference/<case>/solver-stats.{ocaml,rust}.json` を出力する。
4. **Impl Registry / Effect Resolver**  
   - `impl_registry.rs` のキー（`trait_path`, `impl_name`, `stage_requirement`）を JSON で比較し、登録順序を `IndexMap` で固定。  
   - `Type_inference_effect` の `residual_effects` は ASCII ソート済みで記録し、`collect-iterator-audit-metrics.py --section effects` の許容差分 0.5pt 以内を受入基準とする。

## 3. テスト入力セット

| グループ | 由来テスト | ケース例 | 目的 |
| --- | --- | --- | --- |
| `patterns` | `compiler/ocaml/tests/test_type_inference.ml` | `let (x, y) = tuple_fn()` | パターン束縛・row 多相 |
| `callconv` | `tests/test_cli_callconv_snapshot.ml` | `#[callconv("ffi")] fn host()` | CallConv / FFI 対応 |
| `ffi-contract` | `tests/test_ffi_contract.ml` | `extern fn c_abi(x: i32)` | `impl_registry` / ABI 確認 |
| `diagnostics` | `tests/test_cli_diagnostics.ml` | 効果ステージ違反 | `diagnostics.effect_stage_consistency` の確認 |
| `stress` | `examples/cli/*.reml`（複合演算） | `effect pipeline` | ソルバの複雑制約および metrics |

各グループのケース一覧は `reports/dual-write/front-end/w3-type-inference/README.md` に記載する（最低 3 ケース/グループ）。`DUALWRITE_CASES_FILE` 形式は `name::file::<relative-path>` を推奨。
ケース定義ファイルの初期セットは `docs/plans/rust-migration/appendix/w3-dualwrite-cases.txt` に保存し、`scripts/poc_dualwrite_compare.sh --cases` へ直接指定できる。

## 4. CLI / スクリプト更新ポイント
- `compiler/rust/frontend/src/bin/poc_frontend.rs` に `--emit typed-ast --emit constraints --emit typeck-debug <dir>` を実装し、Dual-write 実行時は `--dualwrite-root`/`--dualwrite-run-label`/`--dualwrite-case-label` と組み合わせる。
- `compiler/ocaml/src/cli/` 側でも `--emit-constraints-json <path>` を追加し、Rust 側と同一 JSON スキーマを出力。
- `scripts/poc_dualwrite_compare.sh` へ `--mode typeck` を追加し、`--emit typeck` を前提とした追加成果物（`typed-ast`, `constraints`, `impl-registry`, `effects-metrics`, `typeck-debug`）を収集する。既定では AST/診断の比較のみを実行し、`--mode typeck` 指定時に型推論ログを付加する。
- 失敗時ログは `case/<name>/typeck/` に `stderr.log`, `command.json` を保存し、再実行手順を `summary.md` に追記する。

## 5. 受入基準
1. Dual-write Typed AST / Constraint / Impl Registry JSON が完全一致（差分 0）。  
2. `collect-iterator-audit-metrics.py --section effects --require-success` が pass し、`effects.impl_resolve.delta` が ±0.5pt 以内。  
3. `scripts/validate-diagnostic-json.sh` を W3 ケースに適用した際、型推論エラー由来の診断が全て Schema 合格。  
4. `p1-front-end-checklists.csv` の「制約ソルバ」行に追記した成果物 (`typeck_config.md`, `rust_type_inference_tests.rs`, `effects-metrics.json`) の受入基準が満たされていることを確認できるレポート（`summary.md`）が存在する。

## 6. フォローアップとリスク
- `constraint_solver.ml` 特有の `deferred_impl` / `dictionary_injection` など未実装のブランチは `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に転記し、Rust 実装の TODO コメントとリンクさせる。
- JSON スキーマが変更された場合は `docs/spec/1-2-types-Inference.md` と `docs/spec/3-6-core-diagnostics-audit.md` にも追記が必要。`docs-migrations.log` へ記録して Phase P2 へ引き継ぐ。
- `impl_registry` の determinism を壊す変更（`HashMap` など非順序構造の導入）がレビュー時に検知できるよう、`cargo test typeck::registry::tests::ensures_deterministic_order`（仮）を追加予定。
